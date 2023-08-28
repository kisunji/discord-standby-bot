package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"sync"

	"github.com/bwmarrin/discordgo"
)

var (
	BotToken      = *flag.String("token", "", "Bot access token")
	AppID         = *flag.String("app-id", "", "App ID")
	GuildID       = *flag.String("guild-id", "", "Guild ID (Server ID)")
	StandbyRoleID = *flag.String("standby-role-id", "", "Standby Role ID")
	ChannelID     = *flag.String("channel-id", "", "Channel ID")
)

var (
	activeMessageID = ""
	mu              sync.Mutex
)

func init() {
	flag.Parse()
}

func main() {
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

	cmd, err := discord.ApplicationCommandCreate(AppID, GuildID, &discordgo.ApplicationCommand{
		Name:        "standby",
		Description: "Toggle the 5-Stack Standby role",
	})
	if err != nil {
		panic(err)
	}
	defer discord.ApplicationCommandDelete(AppID, GuildID, cmd.ID)

	remove := discord.AddHandler(func(s *discordgo.Session, i *discordgo.InteractionCreate) {
		if i.ApplicationCommandData().Name == "standby" {
			userID := i.Member.User.ID
			m, err := s.GuildMember(GuildID, userID)
			if err != nil {
				log.Printf("error fetching member: %v\n", err)
			}
			var isStandby bool
			for _, r := range m.Roles {
				if r == StandbyRoleID {
					isStandby = true
					break
				}
			}

			if isStandby {
				// toggle it off
				if err := s.GuildMemberRoleRemove(GuildID, userID, StandbyRoleID); err != nil {
					log.Printf("error removing role: %v\n", err)
					return
				}
				s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
					Type: discordgo.InteractionResponseChannelMessageWithSource,
					Data: &discordgo.InteractionResponseData{
						Content: "You have been removed from standby.",
						Flags:   discordgo.MessageFlagsEphemeral,
					},
				})

				checkStandbyCount(s)
			} else {
				// toggle it on
				if err := s.GuildMemberRoleAdd(GuildID, userID, StandbyRoleID); err != nil {
					log.Printf("error adding role: %v\n", err)
					return
				}
				s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
					Type: discordgo.InteractionResponseChannelMessageWithSource,
					Data: &discordgo.InteractionResponseData{
						Content: "You have been added to standby. Type /standby again to remove yourself.",
						Flags:   discordgo.MessageFlagsEphemeral,
					},
				})

				checkStandbyCount(s)
			}
		}
	})
	defer remove()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt)
	log.Println("Press ctrl+c to exit")
	<-stop

	log.Println("exiting")
}

func checkStandbyCount(s *discordgo.Session) {
	members, err := s.GuildMembers(GuildID, "", 1000)
	if err != nil {
		log.Printf("error fetching members: %v\n", err)
	}
	var standbyCount int
	for _, m := range members {
		for _, r := range m.Roles {
			if r == StandbyRoleID {
				standbyCount++
			}
		}
	}
	if standbyCount >= 6 { // account for the bot
		mu.Lock()
		defer mu.Unlock()

		if activeMessageID != "" {
			// Avoid spamming channel
			return
		}
		m, err := s.ChannelMessageSend(ChannelID, fmt.Sprintf("<@&%s> there are enough members for a game!", StandbyRoleID))
		if err != nil {
			log.Printf("error sending channel message: %v\n", err)
			return
		}
		activeMessageID = m.ID
	} else {
		mu.Lock()
		defer mu.Unlock()

		if activeMessageID != "" {
			if err := s.ChannelMessageDelete(ChannelID, activeMessageID); err != nil {
				log.Printf("error deleting active message: %v", err)
			}
		}
		activeMessageID = ""
	}
}
