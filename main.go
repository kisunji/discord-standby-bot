package main

import (
	"fmt"
	"log"
	"math/rand/v2"
	"net"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/bwmarrin/discordgo"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// Constants
const (
	MaxQueueSize     = 5
	OneMoreThreshold = 4
	QueueTitle       = "5-Stack Standby Queue"
	QueueColor       = 0x0099FF
	MetricsPort      = ":2112"
	ServerPort       = "0.0.0.0:8080"
)

// Button IDs
const (
	JoinQueueButtonID  = "join_queue"
	LeaveQueueButtonID = "leave_queue"
	CloseQueueButtonID = "close_queue"
	OpenQueueButtonID  = "open_queue"
)

// Action types
const (
	ActionJoin          = "join"
	ActionLeave         = "leave"
	ActionJoinWaitlist  = "join_waitlist"
	ActionLeaveWaitlist = "leave_waitlist"
)

// Configuration loaded from environment variables
type Config struct {
	BotToken    string
	AppID       string
	GuildID     string
	AdminRoleID string
	ChannelID   string
}

func loadConfig() *Config {
	return &Config{
		BotToken:    os.Getenv("DISCORD_BOT_TOKEN"),
		AppID:       os.Getenv("STANDBY_APP_ID"),
		GuildID:     os.Getenv("STANDBY_GUILD_ID"),
		AdminRoleID: os.Getenv("STANDBY_ADMIN_ID"),
		ChannelID:   os.Getenv("STANDBY_CHANNEL_ID"),
	}
}

var (
	commandDuration = prometheus.NewHistogram(
		prometheus.HistogramOpts{
			Name:    "command_duration_seconds",
			Help:    "Duration of commands in seconds",
			Buckets: prometheus.DefBuckets,
		},
	)
)

func init() {
	prometheus.MustRegister(commandDuration)
}

func main() {
	config := loadConfig()

	l, err := net.Listen("tcp4", ServerPort)
	if err != nil {
		panic(err)
	}
	defer l.Close()

	discord, err := discordgo.New("Bot " + config.BotToken)
	if err != nil {
		panic(err)
	}
	if err := discord.Open(); err != nil {
		panic(err)
	}
	defer discord.Close()

	if err := discord.UpdateStatusComplex(discordgo.UpdateStatusData{
		Status: "idle",
		Activities: []*discordgo.Activity{
			{
				Name:  "Type /standby",
				Type:  discordgo.ActivityTypeCustom,
				State: "Type /standby to join",
			},
		},
	}); err != nil {
		panic(err)
	}

	{
		cmd, err := discord.ApplicationCommandCreate(config.AppID, config.GuildID, &discordgo.ApplicationCommand{
			Name:        "standby",
			Description: "Open standby queue",
		})
		if err != nil {
			panic(err)
		}
		defer discord.ApplicationCommandDelete(config.AppID, config.GuildID, cmd.ID)
	}
	{
		cmd, err := discord.ApplicationCommandCreate(config.AppID, config.GuildID, &discordgo.ApplicationCommand{
			Name:        "standby-close",
			Description: "Admin command to close existing standby",
		})
		if err != nil {
			panic(err)
		}
		defer discord.ApplicationCommandDelete(config.AppID, config.GuildID, cmd.ID)
	}

	q := NewQueueState(config)

	remove := discord.AddHandler(func(s *discordgo.Session, i *discordgo.InteractionCreate) {
		start := time.Now()
		defer func() {
			duration := time.Since(start).Seconds()
			commandDuration.Observe(duration)
		}()
		switch i.Type {
		case discordgo.InteractionApplicationCommand:
			q.handleSlashCommand(s, i)
		case discordgo.InteractionMessageComponent:
			q.handleButtonClick(s, i)
		}
	})
	defer remove()

	log.Println("Press ctrl+c to exit")
	http.Handle("/metrics", promhttp.Handler())
	http.ListenAndServe(MetricsPort, nil)

	log.Println("exiting")
}

type queueState struct {
	sync.Mutex
	config *Config

	currentMsgID string
	notifyMsgID  string
	oneMoreMsgID string

	lastUser   *discordgo.User
	lastAction string

	users    []*discordgo.User
	waitlist []*discordgo.User
}

// NewQueueState creates a new queue state with the given configuration
func NewQueueState(config *Config) *queueState {
	return &queueState{
		config: config,
	}
}

// lock must be held
func (q *queueState) buildStringLocked() string {
	var sb strings.Builder
	switch q.lastAction {
	case ActionJoin:
		sb.WriteString(fmt.Sprintf("<@%s> joined!\n", q.lastUser.ID))
	case ActionLeave:
		sb.WriteString(fmt.Sprintf("<@%s> left!\n", q.lastUser.ID))
	}
	sb.WriteString(fmt.Sprintf("### Queued users (%d):\n", len(q.users)))
	for _, user := range q.users {
		sb.WriteString(fmt.Sprintf("<@%s>\n", user.ID))
	}

	if len(q.waitlist) > 0 {
		sb.WriteString(fmt.Sprintf("\n### Waitlist (%d):\n", len(q.waitlist)))
		for _, user := range q.waitlist {
			sb.WriteString(fmt.Sprintf("<@%s>\n", user.ID))
		}
	}

	return sb.String()
}

// Helper method to create a queue embed
func (q *queueState) createQueueEmbed(description string) *discordgo.MessageEmbed {
	return &discordgo.MessageEmbed{
		Type:        discordgo.EmbedTypeRich,
		Title:       QueueTitle,
		Color:       QueueColor,
		Description: description,
	}
}

// Helper method to create queue buttons
func (q *queueState) createQueueButtons(disabled bool) []discordgo.MessageComponent {
	return []discordgo.MessageComponent{
		discordgo.ActionsRow{
			Components: []discordgo.MessageComponent{
				discordgo.Button{
					Label:    "Join",
					Style:    discordgo.PrimaryButton,
					CustomID: JoinQueueButtonID,
					Disabled: disabled,
				},
				discordgo.Button{
					Label:    "Leave",
					Style:    discordgo.DangerButton,
					CustomID: LeaveQueueButtonID,
					Disabled: disabled,
				},
				discordgo.Button{
					Label:    "Close",
					Style:    discordgo.SecondaryButton,
					CustomID: CloseQueueButtonID,
				},
			},
		},
	}
}

// Helper method to create closed queue buttons
func (q *queueState) createClosedQueueButtons() []discordgo.MessageComponent {
	return []discordgo.MessageComponent{
		discordgo.ActionsRow{
			Components: []discordgo.MessageComponent{
				discordgo.Button{
					Label:    "Join",
					Style:    discordgo.PrimaryButton,
					CustomID: JoinQueueButtonID,
					Disabled: true,
				},
				discordgo.Button{
					Label:    "Leave",
					Style:    discordgo.DangerButton,
					CustomID: LeaveQueueButtonID,
					Disabled: true,
				},
				discordgo.Button{
					Label:    "Open",
					Style:    discordgo.SecondaryButton,
					CustomID: OpenQueueButtonID,
				},
			},
		},
	}
}

// Helper method to check if user is in queue or waitlist
func (q *queueState) isUserInQueueOrWaitlist(userID string) bool {
	for _, user := range q.users {
		if user.ID == userID {
			return true
		}
	}
	for _, user := range q.waitlist {
		if user.ID == userID {
			return true
		}
	}
	return false
}

// Helper method to respond with ephemeral message
func respondEphemeral(s *discordgo.Session, i *discordgo.InteractionCreate, content string) {
	s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: content,
			Flags:   discordgo.MessageFlagsEphemeral,
		},
	})
}

func (q *queueState) handleSlashCommand(s *discordgo.Session, i *discordgo.InteractionCreate) {
	switch i.ApplicationCommandData().Name {
	case "standby":
		q.Lock()
		defer q.Unlock()

		if q.currentMsgID != "" {
			respondEphemeral(s, i, "There is already an existing queue.")
			return
		}

		if err := q.openQueueLocked(s); err != nil {
			log.Printf("error opening queue: %v", err)
			return
		}

		respondEphemeral(s, i, "Starting queue.")

	case "standby-close":
		userID := i.Member.User.ID
		m, err := s.GuildMember(q.config.GuildID, userID)
		if err != nil {
			log.Printf("error fetching member: %v\n", err)
		}
		var isAdmin bool
		for _, r := range m.Roles {
			if r == q.config.AdminRoleID {
				isAdmin = true
				break
			}
		}
		if !isAdmin {
			respondEphemeral(s, i, "Only admins can use this command.")
		} else {
			q.Lock()
			defer q.Unlock()

			if q.currentMsgID == "" {
				respondEphemeral(s, i, "No active queue to close.")
			}
			q.closeQueueLocked(s)

			respondEphemeral(s, i, "Closing queue.")
		}
	}
}

// lock must be held
func (q *queueState) openQueueLocked(s *discordgo.Session) error {
	msg, err := s.ChannelMessageSendComplex(q.config.ChannelID, &discordgo.MessageSend{
		Embeds:     []*discordgo.MessageEmbed{q.createQueueEmbed(q.buildStringLocked())},
		Components: q.createQueueButtons(false),
	})
	if err != nil {
		return err
	}
	q.currentMsgID = msg.ID
	return nil
}

// lock must be held
func (q *queueState) closeQueueLocked(s *discordgo.Session) {
	closedButtons := q.createClosedQueueButtons()
	_, err := s.ChannelMessageEditComplex(&discordgo.MessageEdit{
		ID:         q.currentMsgID,
		Channel:    q.config.ChannelID,
		Embeds:     &[]*discordgo.MessageEmbed{q.createQueueEmbed("Queue is closed")},
		Components: &closedButtons,
	})
	if err != nil {
		log.Printf("error editing message closing queue: %v", err)
	}

	q.currentMsgID = ""
	q.lastAction = ""
	q.lastUser = nil
	q.users = nil
	q.waitlist = nil
	if q.notifyMsgID != "" {
		if err := s.ChannelMessageDelete(q.config.ChannelID, q.notifyMsgID); err != nil {
			log.Printf("error deleting active message: %v\n", err)
		}
	}
	q.notifyMsgID = ""
}

func (q *queueState) handleButtonClick(s *discordgo.Session, i *discordgo.InteractionCreate) {
	q.Lock()
	defer q.Unlock()

	s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseDeferredMessageUpdate,
	})

	switch i.MessageComponentData().CustomID {
	case CloseQueueButtonID:
		q.closeQueueLocked(s)
		return
	case OpenQueueButtonID:
		// Add the user who opened queue
		q.users = append(q.users, i.Member.User)
		q.lastUser = i.Member.User
		q.lastAction = ActionJoin

		q.openQueueLocked(s)

		// Delete the original message to clean up clutter
		if err := s.ChannelMessageDelete(q.config.ChannelID, i.Message.ID); err != nil {
			log.Printf("error deleting active message: %v\n", err)
		}
		return
	case JoinQueueButtonID:
		// Check if user is already in queue or waitlist
		if q.isUserInQueueOrWaitlist(i.Member.User.ID) {
			return
		}

		// If queue has space, add to queue; otherwise add to waitlist
		if len(q.users) < MaxQueueSize {
			q.users = append(q.users, i.Member.User)
			q.lastUser = i.Member.User
			q.lastAction = ActionJoin
		} else {
			q.waitlist = append(q.waitlist, i.Member.User)
			q.lastUser = i.Member.User
			q.lastAction = ActionJoinWaitlist
		}
	case LeaveQueueButtonID:
		// Check if user is in the main queue
		for idx, user := range q.users {
			if user.ID == i.Member.User.ID {
				q.users = append(q.users[:idx], q.users[idx+1:]...)
				q.lastUser = i.Member.User
				q.lastAction = ActionLeave

				// Move first waitlisted user to queue if waitlist exists
				if len(q.waitlist) > 0 {
					promoted := q.waitlist[0]
					q.waitlist = q.waitlist[1:]
					q.users = append(q.users, promoted)

					// Notify the promoted user
					_, err := s.ChannelMessageSend(q.config.ChannelID, fmt.Sprintf("🎉 <@%s> you've been moved from the waitlist to the queue!", promoted.ID))
					if err != nil {
						log.Printf("error sending promotion notification: %v\n", err)
					}
				}
				break
			}
		}

		// Check if user is in the waitlist
		for idx, user := range q.waitlist {
			if user.ID == i.Member.User.ID {
				q.waitlist = append(q.waitlist[:idx], q.waitlist[idx+1:]...)
				q.lastUser = i.Member.User
				q.lastAction = ActionLeaveWaitlist
				break
			}
		}
	}

	// Update the queue message
	activeButtons := q.createQueueButtons(false)
	_, err := s.ChannelMessageEditComplex(&discordgo.MessageEdit{
		ID:         q.currentMsgID,
		Channel:    q.config.ChannelID,
		Embeds:     &[]*discordgo.MessageEmbed{q.createQueueEmbed(q.buildStringLocked())},
		Components: &activeButtons,
	})
	if err != nil {
		log.Printf("error editing message handling button click: %v", err)
		return
	}

	// Close queue if a user leaving would leave it at 0
	if len(q.users) == 0 {
		q.closeQueueLocked(s)
	}

	if len(q.users) == OneMoreThreshold {
		m, err := s.ChannelMessageSend(q.config.ChannelID, getRandomOneMore())
		if err != nil {
			log.Printf("error sending channel message: %v\n", err)
			return
		}
		q.oneMoreMsgID = m.ID
	} else {
		if q.oneMoreMsgID != "" {
			if err := s.ChannelMessageDelete(q.config.ChannelID, q.oneMoreMsgID); err != nil {
				log.Printf("error deleting active message: %v\n", err)
			}
		}
		q.oneMoreMsgID = ""
	}

	if len(q.users) == MaxQueueSize && q.notifyMsgID == "" {
		usernames := make([]string, len(q.users))
		for i, user := range q.users {
			usernames[i] = fmt.Sprintf("<@%s>", user.ID)
		}

		m, err := s.ChannelMessageSend(q.config.ChannelID, fmt.Sprintf("There are enough users for a game! %s", strings.Join(usernames, ", ")))
		if err != nil {
			log.Printf("error sending channel message: %v\n", err)
			return
		}
		q.notifyMsgID = m.ID
	} else if len(q.users) < MaxQueueSize && q.notifyMsgID != "" {
		if err := s.ChannelMessageDelete(q.config.ChannelID, q.notifyMsgID); err != nil {
			log.Printf("error deleting active message: %v\n", err)
		}
		q.notifyMsgID = ""
	}
}

func getRandomOneMore() string {
	translations := []string{
		"nog een", "edhe një", "አንደኛ ተጨማሪ", "واحد آخر", "ևս մեկը", "bir daha",
		"beste bat", "яшчэ адзін", "আরেকটি", "još jedan", "още един", "un més",
		"usa pa", "再一个", "再一個", "još jedan", "ještě jeden", "en mere",
		"nog een", "one more", "ankoraŭ unu", "veel üks", "isa pa", "vielä yksi",
		"encore un", "un máis", "კიდევ ერთი", "noch eins", "ένα ακόμα", "એક વધુ",
		"yon lòt", "ɗaya kuma", "עוד אחד", "एक और", "ib ntxiv", "még egy",
		"einn í viðbót", "otu ọzọ", "satu lagi", "ceann eile", "un altro", "もう一つ",
		"siji maneh", "ಇನ್ನೊಂದು", "тағы бір", "មួយទៀត", "undi umwe", "하나 더",
		"yek din", "дагы бир", "ອີກໜຶ່ງ", "unum magis", "vēl viens", "dar vienas",
		"nach eng", "уште еден", "iray hafa", "satu lagi", "മറ്റൊന്ന്", "ieħor",
		"kotahi atu", "आणखी एक", "дахин нэг", "တစ်ခုထပ်", "अर्को", "en til",
		"ଆଉ ଗୋଟିଏ", "یو بل", "یکی دیگر", "jeszcze jeden", "mais um", "ਇੱਕ ਹੋਰ",
		"încă unul", "еще один", "tasi le isi", "fear eile", "још један", "e 'ngoe hape",
		"chimwe zvakare", "هڪ وڌيڪ", "තවත් එකක්", "ešte jeden", "še en", "mid kale",
		"uno más", "hiji deui", "moja zaidi", "en till", "боз як", "இன்னொரு",
		"тагын бер", "మరోటి", "อีกหนึ่ง", "bir tane daha", "ýene bir", "ще один",
		"ایک اور", "تېخىمۇ بىر", "yana bitta", "một cái nữa", "un arall", "enye",
		"נאָך איינער", "ọkan siwaju sii", "elilodwa elengeziwe",
	}

	// Get random translation
	return translations[rand.IntN(len(translations))]
}
