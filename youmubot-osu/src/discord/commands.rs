use super::*;
use display::display_beatmapset;
use embeds::ScoreEmbedBuilder;
use link_parser::EmbedType;
use poise::CreateReply;
use serenity::all::User;

/// osu!-related command group.
#[poise::command(
    slash_command,
    subcommands("profile", "top", "recent", "pinned", "save", "forcesave", "beatmap")
)]
pub async fn osu<U: HasOsuEnv>(_ctx: CmdContext<'_, U>) -> Result<()> {
    Ok(())
}

/// Returns top plays for a given player.
///
/// If no osu! username is given, defaults to the currently registered user.
#[poise::command(slash_command)]
async fn top<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "Index of the score"]
    #[min = 1]
    #[max = 100]
    index: Option<u8>,
    #[description = "Score listing style"] style: Option<ScoreListStyle>,
    #[description = "Game mode"] mode: Option<Mode>,
    #[description = "osu! username"] username: Option<String>,
    #[description = "Discord username"] discord_name: Option<User>,
) -> Result<()> {
    let env = ctx.data().osu_env();
    let username_arg = arg_from_username_or_discord(username, discord_name);
    let args = ListingArgs::from_params(
        env,
        index,
        style.unwrap_or(ScoreListStyle::Table),
        mode,
        username_arg,
        ctx.author().id,
    )
    .await?;

    ctx.defer().await?;
    let osu_client = &env.client;
    let mut plays = osu_client
        .user_best(UserID::ID(args.user.id), |f| f.mode(args.mode).limit(100))
        .await?;

    plays.sort_unstable_by(|a, b| b.pp.partial_cmp(&a.pp).unwrap());

    handle_listing(ctx, plays, args, |nth, b| b.top_record(nth), "top").await
}

/// Get an user's profile.
#[poise::command(slash_command)]
async fn profile<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "Game mode"]
    #[rename = "mode"]
    mode_override: Option<Mode>,
    #[description = "osu! username"] username: Option<String>,
    #[description = "Discord username"] discord_name: Option<User>,
) -> Result<()> {
    let env = ctx.data().osu_env();
    let username_arg = arg_from_username_or_discord(username, discord_name);
    let (mode, user) = user_header_or_default_id(username_arg, env, ctx.author().id).await?;
    let mode = mode_override.unwrap_or(mode);

    ctx.defer().await?;

    let user = env
        .client
        .user(&UserID::ID(user.id), |f| f.mode(mode))
        .await?;

    match user {
        Some(u) => {
            let ex = UserExtras::from_user(env, &u, mode).await?;
            ctx.send(
                CreateReply::default()
                    .content(format!("Here is {}'s **{}** profile!", u.mention(), mode))
                    .embed(user_embed(u, ex)),
            )
            .await?;
        }
        None => {
            ctx.reply("🔍 user not found!").await?;
        }
    };
    Ok(())
}

/// Returns recent plays from a given player.
///
/// If no osu! username is given, defaults to the currently registered user.
#[poise::command(slash_command)]
async fn recent<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "Index of the score"]
    #[min = 1]
    #[max = 50]
    index: Option<u8>,
    #[description = "Score listing style"] style: Option<ScoreListStyle>,
    #[description = "Game mode"] mode: Option<Mode>,
    #[description = "osu! username"] username: Option<String>,
    #[description = "Discord username"] discord_name: Option<User>,
) -> Result<()> {
    let env = ctx.data().osu_env();
    let args = arg_from_username_or_discord(username, discord_name);
    let style = style.unwrap_or(ScoreListStyle::Table);

    let args = ListingArgs::from_params(env, index, style, mode, args, ctx.author().id).await?;

    ctx.defer().await?;

    let osu_client = &env.client;
    let plays = osu_client
        .user_recent(UserID::ID(args.user.id), |f| f.mode(args.mode).limit(50))
        .await?;

    handle_listing(ctx, plays, args, |_, b| b, "recent").await
}

/// Returns pinned plays from a given player.
///
/// If no osu! username is given, defaults to the currently registered user.
#[poise::command(slash_command)]
async fn pinned<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "Index of the score"]
    #[min = 1]
    #[max = 50]
    index: Option<u8>,
    #[description = "Score listing style"] style: Option<ScoreListStyle>,
    #[description = "Game mode"] mode: Option<Mode>,
    #[description = "osu! username"] username: Option<String>,
    #[description = "Discord username"] discord_name: Option<User>,
) -> Result<()> {
    let env = ctx.data().osu_env();
    let args = arg_from_username_or_discord(username, discord_name);
    let style = style.unwrap_or(ScoreListStyle::Table);

    let args = ListingArgs::from_params(env, index, style, mode, args, ctx.author().id).await?;

    ctx.defer().await?;

    let osu_client = &env.client;
    let plays = osu_client
        .user_pins(UserID::ID(args.user.id), |f| f.mode(args.mode).limit(50))
        .await?;

    handle_listing(ctx, plays, args, |_, b| b, "pinned").await
}

/// Save your osu! profile into Youmu's database for tracking and quick commands.
#[poise::command(slash_command)]
pub async fn save<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "The osu! username to set to"] username: String,
) -> Result<()> {
    let env = ctx.data().osu_env();
    ctx.defer().await?;
    let (u, mode, score, beatmap, info) = find_save_requirements(env, username).await?;
    let reply = ctx
        .clone()
        .send(
            CreateReply::default()
                .content(format!(
                    "To set your osu username to **{}**, please make your most recent play \
            be the following map: `/b/{}` in **{}** mode! \
        It does **not** have to be a pass, and **NF** can be used! \
        React to this message with 👌 within 5 minutes when you're done!",
                    u.username,
                    score.beatmap_id,
                    mode.as_str_new_site()
                ))
                .embed(beatmap_embed(&beatmap, mode, Mods::NOMOD, &info))
                .components(vec![beatmap_components(mode, ctx.guild_id())]),
        )
        .await?
        .into_message()
        .await?;
    handle_save_respond(
        ctx.serenity_context(),
        &env,
        ctx.author().id,
        reply,
        &beatmap,
        u,
        mode,
    )
    .await?;
    Ok(())
}

/// Force-save an osu! profile into Youmu's database for tracking and quick commands.
#[poise::command(slash_command, owners_only)]
pub async fn forcesave<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "The osu! username to set to"] username: String,
    #[description = "The discord user to assign to"] discord_name: User,
) -> Result<()> {
    let env = ctx.data().osu_env();
    let osu_client = &env.client;
    ctx.defer().await?;
    let Some(u) = osu_client
        .user(&UserID::from_string(username.clone()), |f| f)
        .await?
    else {
        return Err(Error::msg("osu! user not found"));
    };
    add_user(discord_name.id, &u, &env).await?;
    let ex = UserExtras::from_user(&env, &u, u.preferred_mode).await?;
    ctx.send(
        CreateReply::default()
            .content(
                MessageBuilder::new()
                    .push("Youmu is now tracking user ")
                    .push(discord_name.mention().to_string())
                    .push(" with osu! account ")
                    .push_bold_safe(username)
                    .build(),
            )
            .embed(user_embed(u, ex)),
    )
    .await?;
    Ok(())
}

async fn handle_listing<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    plays: Vec<Score>,
    listing_args: ListingArgs,
    transform: impl for<'a> Fn(u8, ScoreEmbedBuilder<'a>) -> ScoreEmbedBuilder<'a>,
    listing_kind: &'static str,
) -> Result<()> {
    let env = ctx.data().osu_env();
    let ListingArgs {
        nth,
        style,
        mode,
        user,
    } = listing_args;

    match nth {
        Nth::Nth(nth) => {
            let Some(play) = plays.get(nth as usize) else {
                Err(Error::msg("no such play"))?
            };

            let beatmap = env.beatmaps.get_beatmap(play.beatmap_id, mode).await?;
            let content = env.oppai.get_beatmap(beatmap.beatmap_id).await?;
            let beatmap = BeatmapWithMode(beatmap, mode);

            ctx.send({
                CreateReply::default()
                    .content(format!(
                        "Here is the #{} {} play by {}!",
                        nth + 1,
                        listing_kind,
                        user.mention()
                    ))
                    .embed({
                        let mut b =
                            transform(nth + 1, score_embed(&play, &beatmap, &content, user));
                        if let Some(rank) = play.global_rank {
                            b = b.world_record(rank as u16);
                        }
                        b.build()
                    })
                    .components(vec![score_components(ctx.guild_id())])
            })
            .await?;

            // Save the beatmap...
            cache::save_beatmap(&env, ctx.channel_id(), &beatmap).await?;
        }
        Nth::All => {
            let reply = ctx
                .clone()
                .reply(format!(
                    "Here are the {} plays by {}!",
                    listing_kind,
                    user.mention()
                ))
                .await?
                .into_message()
                .await?;
            style
                .display_scores(plays, mode, ctx.serenity_context(), ctx.guild_id(), reply)
                .await?;
        }
    }
    Ok(())
}

/// Get information about a beatmap, or the last beatmap mentioned in the channel.
#[poise::command(slash_command)]
async fn beatmap<U: HasOsuEnv>(
    ctx: CmdContext<'_, U>,
    #[description = "A link or shortlink to the beatmap or beatmapset"] map: Option<String>,
    #[description = "Override the mods on the map"] mods: Option<UnparsedMods>,
    #[description = "Override the mode of the map"] mode: Option<Mode>,
    #[description = "Load the beatmapset instead"] beatmapset: Option<bool>,
) -> Result<()> {
    let env = ctx.data().osu_env();

    ctx.defer().await?;

    let beatmap = match map {
        None => {
            let Some((BeatmapWithMode(b, mode), bmmods)) =
                load_beatmap(env, ctx.channel_id(), None as Option<&'_ Message>).await
            else {
                return Err(Error::msg("no beatmap mentioned in this channel"));
            };
            let mods = bmmods.unwrap_or_else(|| Mods::NOMOD.clone());
            let info = env
                .oppai
                .get_beatmap(b.beatmap_id)
                .await?
                .get_possible_pp_with(mode, &mods);
            EmbedType::Beatmap(Box::new(b), info, mods)
        }
        Some(map) => {
            let Some(results) = stream::select(
                link_parser::parse_new_links(env, &map),
                stream::select(
                    link_parser::parse_old_links(env, &map),
                    link_parser::parse_short_links(env, &map),
                ),
            )
            .next()
            .await
            else {
                return Err(Error::msg("no beatmap detected in the argument"));
            };
            results.embed
        }
    };

    // override into beatmapset if needed
    let beatmap = if beatmapset == Some(true) {
        match beatmap {
            EmbedType::Beatmap(beatmap, _, _) => {
                let beatmaps = env.beatmaps.get_beatmapset(beatmap.beatmapset_id).await?;
                EmbedType::Beatmapset(beatmaps)
            }
            bm @ EmbedType::Beatmapset(_) => bm,
        }
    } else {
        beatmap
    };

    // override mods and mode if needed
    match beatmap {
        EmbedType::Beatmap(beatmap, info, bmmods) => {
            let (beatmap, info, mods) = if mods.is_none() && mode.is_none_or(|v| v == beatmap.mode)
            {
                (*beatmap, info, bmmods)
            } else {
                let mode = mode.unwrap_or(beatmap.mode);
                let mods = match mods {
                    None => bmmods,
                    Some(mods) => mods.to_mods(mode)?,
                };
                let beatmap = env.beatmaps.get_beatmap(beatmap.beatmap_id, mode).await?;
                let info = env
                    .oppai
                    .get_beatmap(beatmap.beatmap_id)
                    .await?
                    .get_possible_pp_with(mode, &mods);
                (beatmap, info, mods)
            };
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "Information for beatmap `{}`",
                        beatmap.short_link(mode, &mods)
                    ))
                    .embed(beatmap_embed(
                        &beatmap,
                        mode.unwrap_or(beatmap.mode),
                        &mods,
                        &info,
                    ))
                    .components(vec![beatmap_components(
                        mode.unwrap_or(beatmap.mode),
                        ctx.guild_id(),
                    )]),
            )
            .await?;
        }
        EmbedType::Beatmapset(vec) => {
            let b0 = &vec[0];
            let msg = ctx
                .clone()
                .reply(format!(
                    "Information for beatmapset [`/s/{}`](<{}>)",
                    b0.beatmapset_id,
                    b0.beatmapset_link()
                ))
                .await?
                .into_message()
                .await?;
            display_beatmapset(
                ctx.serenity_context().clone(),
                vec,
                mode,
                mods,
                ctx.guild_id(),
                msg,
            )
            .await?;
        }
    };

    Ok(())
}

fn arg_from_username_or_discord(
    username: Option<String>,
    discord_name: Option<User>,
) -> Option<UsernameArg> {
    match (username, discord_name) {
        (Some(v), _) => Some(UsernameArg::Raw(v)),
        (_, Some(u)) => Some(UsernameArg::Tagged(u.id)),
        (None, None) => None,
    }
}
