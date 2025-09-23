package main

import (
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/bwmarrin/discordgo"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"golang.org/x/exp/rand"
)

var (
	BotToken    = os.Getenv("DISCORD_BOT_TOKEN")
	AppID       = os.Getenv("STANDBY_APP_ID")
	GuildID     = os.Getenv("STANDBY_GUILD_ID")
	AdminRoleID = os.Getenv("STANDBY_ADMIN_ID")
	ChannelID   = os.Getenv("STANDBY_CHANNEL_ID")
)

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
	l, err := net.Listen("tcp4", "0.0.0.0:8080")
	if err != nil {
		panic(err)
	}
	defer l.Close()

	discord, err := discordgo.New("Bot " + BotToken)
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
		cmd, err := discord.ApplicationCommandCreate(AppID, GuildID, &discordgo.ApplicationCommand{
			Name:        "standby",
			Description: "Open standby queue",
		})
		if err != nil {
			panic(err)
		}
		defer discord.ApplicationCommandDelete(AppID, GuildID, cmd.ID)
	}
	{
		cmd, err := discord.ApplicationCommandCreate(AppID, GuildID, &discordgo.ApplicationCommand{
			Name:        "standby-close",
			Description: "Admin command to close existing standby",
		})
		if err != nil {
			panic(err)
		}
		defer discord.ApplicationCommandDelete(AppID, GuildID, cmd.ID)
	}

	q := queueState{}

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
	http.ListenAndServe(":2112", nil)

	log.Println("exiting")
}

type queueState struct {
	sync.Mutex

	currentMsgID string
	notifyMsgID  string
	oneMoreMsgID string

	lastUser   *discordgo.User
	lastAction string

	users []*discordgo.User
}

// lock must be held
func (q *queueState) buildStringLocked() string {
	var sb strings.Builder
	switch q.lastAction {
	case "join":
		sb.WriteString(fmt.Sprintf("<@%s> joined queue!\n", q.lastUser.ID))
	case "leave":
		sb.WriteString(fmt.Sprintf("<@%s> left queue!\n", q.lastUser.ID))
	}
	sb.WriteString(fmt.Sprintf("### Queued users (%d):\n", len(q.users)))
	for _, user := range q.users {
		sb.WriteString(fmt.Sprintf("<@%s>\n", user.ID))
	}

	return sb.String()
}

func (q *queueState) handleSlashCommand(s *discordgo.Session, i *discordgo.InteractionCreate) {
	switch i.ApplicationCommandData().Name {
	case "standby":
		q.Lock()
		defer q.Unlock()

		if q.currentMsgID != "" {
			s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
				Type: discordgo.InteractionResponseChannelMessageWithSource,
				Data: &discordgo.InteractionResponseData{
					Content: "There is already an existing queue.",
					Flags:   discordgo.MessageFlagsEphemeral,
				},
			})
			return
		}

		if err := q.openQueueLocked(s); err != nil {
			log.Printf("error opening queue: %v", err)
			return
		}

		s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
			Type: discordgo.InteractionResponseChannelMessageWithSource,
			Data: &discordgo.InteractionResponseData{
				Content: "Starting queue.",
				Flags:   discordgo.MessageFlagsEphemeral,
			},
		})

	case "standby-close":
		userID := i.Member.User.ID
		m, err := s.GuildMember(GuildID, userID)
		if err != nil {
			log.Printf("error fetching member: %v\n", err)
		}
		var isAdmin bool
		for _, r := range m.Roles {
			if r == AdminRoleID {
				isAdmin = true
				break
			}
		}
		if !isAdmin {
			s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
				Type: discordgo.InteractionResponseChannelMessageWithSource,
				Data: &discordgo.InteractionResponseData{
					Content: "Only admins can use this command.",
					Flags:   discordgo.MessageFlagsEphemeral,
				},
			})
		} else {
			q.Lock()
			defer q.Unlock()

			if q.currentMsgID == "" {
				s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
					Type: discordgo.InteractionResponseChannelMessageWithSource,
					Data: &discordgo.InteractionResponseData{
						Content: "No active queue to close.",
						Flags:   discordgo.MessageFlagsEphemeral,
					},
				})
			}
			q.closeQueueLocked(s)

			s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
				Type: discordgo.InteractionResponseChannelMessageWithSource,
				Data: &discordgo.InteractionResponseData{
					Content: "Closing queue.",
					Flags:   discordgo.MessageFlagsEphemeral,
				},
			})
		}
	}
}

// lock must be held
func (q *queueState) openQueueLocked(s *discordgo.Session) error {
	msg, err := s.ChannelMessageSendComplex(ChannelID, &discordgo.MessageSend{
		Embeds: []*discordgo.MessageEmbed{
			{
				Type:        discordgo.EmbedTypeRich,
				Title:       "5-Stack Standby Queue",
				Color:       0x0099FF,
				Description: q.buildStringLocked(),
			},
		},
		Components: []discordgo.MessageComponent{
			discordgo.ActionsRow{
				Components: []discordgo.MessageComponent{
					discordgo.Button{
						Label:    "Join",
						Style:    discordgo.PrimaryButton,
						CustomID: "join_queue",
					},
					discordgo.Button{
						Label:    "Leave",
						Style:    discordgo.DangerButton,
						CustomID: "leave_queue",
					},
					discordgo.Button{
						Label:    "Close",
						Style:    discordgo.SecondaryButton,
						CustomID: "close_queue",
					},
				},
			},
		},
	})
	if err != nil {
		return err
	}
	q.currentMsgID = msg.ID
	return nil
}

// lock must be held
func (q *queueState) closeQueueLocked(s *discordgo.Session) {
	_, err := s.ChannelMessageEditComplex(&discordgo.MessageEdit{
		ID:      q.currentMsgID,
		Channel: ChannelID,
		Embeds: &[]*discordgo.MessageEmbed{
			{
				Type:        discordgo.EmbedTypeRich,
				Title:       "5-Stack Standby Queue",
				Color:       0x0099FF,
				Description: "Queue is closed",
			},
		},
		Components: &[]discordgo.MessageComponent{
			discordgo.ActionsRow{
				Components: []discordgo.MessageComponent{
					discordgo.Button{
						Label:    "Join",
						Style:    discordgo.PrimaryButton,
						CustomID: "join_queue",
						Disabled: true,
					},
					discordgo.Button{
						Label:    "Leave",
						Style:    discordgo.DangerButton,
						CustomID: "leave_queue",
						Disabled: true,
					},
					discordgo.Button{
						Label:    "Open",
						Style:    discordgo.SecondaryButton,
						CustomID: "open_queue",
					},
				},
			},
		},
	})
	if err != nil {
		log.Printf("error editing message closing queue: %v", err)
	}

	q.currentMsgID = ""
	q.lastAction = ""
	q.lastUser = nil
	q.users = nil
	if q.notifyMsgID != "" {
		if err := s.ChannelMessageDelete(ChannelID, q.notifyMsgID); err != nil {
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
	case "close_queue":
		q.closeQueueLocked(s)
		return
	case "open_queue":
		// Add the user who opened queue
		q.users = append(q.users, i.Member.User)
		q.lastUser = i.Member.User
		q.lastAction = "join"

		q.openQueueLocked(s)

		// Delete the original message to clean up clutter
		if err := s.ChannelMessageDelete(ChannelID, i.Message.ID); err != nil {
			log.Printf("error deleting active message: %v\n", err)
		}
		return
	case "join_queue":
		for _, user := range q.users {
			if user.ID == i.Member.User.ID {
				return
			}
		}
		q.users = append(q.users, i.Member.User)
		q.lastUser = i.Member.User
		q.lastAction = "join"
	case "leave_queue":
		for idx, user := range q.users {
			if user.ID == i.Member.User.ID {
				q.users = append(q.users[:idx], q.users[idx+1:]...)
			}
		}
		q.lastUser = i.Member.User
		q.lastAction = "leave"
	}
	_, err := s.ChannelMessageEditComplex(&discordgo.MessageEdit{
		ID:      q.currentMsgID,
		Channel: ChannelID,
		Embeds: &[]*discordgo.MessageEmbed{
			{
				Type:        discordgo.EmbedTypeRich,
				Title:       "5-Stack Standby Queue",
				Color:       0x0099FF,
				Description: q.buildStringLocked(),
			},
		},
		Components: &[]discordgo.MessageComponent{
			discordgo.ActionsRow{
				Components: []discordgo.MessageComponent{
					discordgo.Button{
						Label:    "Join",
						Style:    discordgo.PrimaryButton,
						CustomID: "join_queue",
					},
					discordgo.Button{
						Label:    "Leave",
						Style:    discordgo.DangerButton,
						CustomID: "leave_queue",
					},
					discordgo.Button{
						Label:    "Close",
						Style:    discordgo.SecondaryButton,
						CustomID: "close_queue",
					},
				},
			},
		},
	})
	if err != nil {
		log.Printf("error editing message handling button click: %v", err)
		return
	}

	// Close queue if a user leaving would leave it at 0
	if len(q.users) == 0 {
		q.closeQueueLocked(s)
	}

	if len(q.users) == 4 {
		m, err := s.ChannelMessageSend(ChannelID, getRandomOneMore())
		if err != nil {
			log.Printf("error sending channel message: %v\n", err)
			return
		}
		q.oneMoreMsgID = m.ID
	} else {
		if q.oneMoreMsgID != "" {
			if err := s.ChannelMessageDelete(ChannelID, q.oneMoreMsgID); err != nil {
				log.Printf("error deleting active message: %v\n", err)
			}
		}
		q.oneMoreMsgID = ""
	}

	if len(q.users) >= 5 && q.notifyMsgID == "" {
		usernames := make([]string, len(q.users))
		for i, user := range q.users {
			usernames[i] = fmt.Sprintf("<@%s>", user.ID)
		}

		m, err := s.ChannelMessageSend(ChannelID, fmt.Sprintf("There are enough users for a game! %s", strings.Join(usernames, ", ")))
		if err != nil {
			log.Printf("error sending channel message: %v\n", err)
			return
		}
		q.notifyMsgID = m.ID
	} else {
		if q.notifyMsgID != "" {
			if err := s.ChannelMessageDelete(ChannelID, q.notifyMsgID); err != nil {
				log.Printf("error deleting active message: %v\n", err)
			}
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
	return translations[rand.Intn(len(translations))]
}
