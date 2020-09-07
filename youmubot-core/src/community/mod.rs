use rand::{
    distributions::{Distribution, Uniform},
    thread_rng,
};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError as Error, CommandResult,
    },
    model::{
        channel::{Channel, Message},
        id::RoleId,
        user::OnlineStatus,
    },
    utils::MessageBuilder,
};
use youmubot_prelude::*;

mod roles;
mod votes;

use roles::{ADD_COMMAND, LIST_COMMAND, REMOVE_COMMAND, TOGGLE_COMMAND};
use votes::VOTE_COMMAND;

#[group]
#[description = "Community related commands. Usually comes with some sort of delays, since it involves pinging"]
#[only_in("guilds")]
#[commands(choose, vote, add, list, remove, toggle)]
struct Community;

#[command]
#[description = r"👑 Randomly choose an active member and mention them!
Note that only online/idle users in the channel are chosen from."]
#[usage = "[title = the chosen one] / [limited roles = everyone online]"]
#[example = "the strongest in Gensokyo"]
#[bucket = "community"]
#[max_args(2)]
pub async fn choose(ctx: &Context, m: &Message, mut args: Args) -> CommandResult {
    let title = if args.is_empty() {
        "the chosen one".to_owned()
    } else {
        args.single::<String>()?
    };
    let role = args.single::<RoleId>().ok();

    let users: Result<Vec<_>, Error> = {
        let guild = m.guild(&ctx).await.unwrap();
        let presences = &guild.presences;
        let channel = m.channel_id.to_channel(&ctx).await?;
        if let Channel::Guild(channel) = channel {
            Ok(channel
                .members(&ctx)
                .await?
                .into_iter()
                .filter(|v| !v.user.bot) // Filter out bots
                .filter(|v| {
                    // Filter out only online people
                    presences
                        .get(&v.user.id)
                        .map(|presence| {
                            presence.status == OnlineStatus::Online
                                || presence.status == OnlineStatus::Idle
                        })
                        .unwrap_or(false)
                })
                .map(|mem| future::ready(mem))
                .collect::<stream::FuturesUnordered<_>>()
                .filter_map(|member| async move {
                    // Filter by role if provided
                    if let Some(role) = role {
                        if member
                            .roles(&ctx)
                            .await
                            .map(|roles| roles.into_iter().any(|r| role == r.id))
                            .unwrap_or(false)
                        {
                            Some(member)
                        } else {
                            None
                        }
                    } else {
                        Some(member)
                    }
                })
                .collect()
                .await)
        } else {
            panic!()
        }
    };
    let users = users?;

    if users.len() < 2 {
        m.reply(
            &ctx,
            "🍰 Have this cake for yourself because no-one is here for the gods to pick.",
        )
        .await?;
        return Ok(());
    }

    let winner = {
        let uniform = Uniform::from(0..users.len());
        let mut rng = thread_rng();
        &users[uniform.sample(&mut rng)]
    };

    m.channel_id
        .send_message(&ctx, |c| {
            c.content(
                MessageBuilder::new()
                    .push("👑 The Gensokyo gods have gathered around and decided, out of ")
                    .push_bold(format!("{}", users.len()))
                    .push(" ")
                    .push(
                        role.map(|r| r.mention() + "s")
                            .unwrap_or("potential prayers".to_owned()),
                    )
                    .push(", ")
                    .push(winner.mention())
                    .push(" will be ")
                    .push_bold_safe(title)
                    .push(". Congrats! 🎉 🎊 🥳")
                    .build(),
            )
        })
        .await?;

    Ok(())
}
