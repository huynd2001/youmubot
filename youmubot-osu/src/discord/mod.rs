use std::{str::FromStr, sync::Arc};

use rand::seq::IteratorRandom;
use serenity::{
    builder::{CreateMessage, EditMessage},
    collector,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::Message,
    utils::MessageBuilder,
};

use db::{OsuLastBeatmap, OsuSavedUsers, OsuUser, OsuUserBests};
use embeds::{beatmap_embed, score_embed, user_embed};
use hook::SHORT_LINK_REGEX;
pub use hook::{dot_osu_hook, hook};
use server_rank::{SERVER_RANK_COMMAND, SHOW_LEADERBOARD_COMMAND};
use youmubot_prelude::announcer::AnnouncerHandler;
use youmubot_prelude::{stream::FuturesUnordered, *};

use crate::{
    discord::beatmap_cache::BeatmapMetaCache,
    discord::display::ScoreListStyle,
    discord::oppai_cache::{BeatmapCache, BeatmapInfo},
    models::{Beatmap, Mode, Mods, Score, User},
    request::{BeatmapRequestKind, UserID},
    Client as OsuHttpClient,
};

mod announcer;
pub(crate) mod beatmap_cache;
mod cache;
mod db;
pub(crate) mod display;
pub(crate) mod embeds;
mod hook;
pub(crate) mod oppai_cache;
mod server_rank;

/// The osu! client.
pub(crate) struct OsuClient;

impl TypeMapKey for OsuClient {
    type Value = Arc<OsuHttpClient>;
}

/// The environment for osu! app commands.
#[derive(Clone)]
pub struct OsuEnv {
    pub(crate) prelude: Env,
    // databases
    pub(crate) saved_users: OsuSavedUsers,
    pub(crate) last_beatmaps: OsuLastBeatmap,
    pub(crate) user_bests: OsuUserBests,
    // clients
    pub(crate) client: Arc<OsuHttpClient>,
    pub(crate) oppai: BeatmapCache,
    pub(crate) beatmaps: BeatmapMetaCache,
}

impl std::fmt::Debug for OsuEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<osu::Env>")
    }
}

impl TypeMapKey for OsuEnv {
    type Value = OsuEnv;
}

/// Sets up the osu! command handling section.
///
/// This automatically enables:
///  - Related databases
///  - An announcer system (that will eventually be revamped)
///  - The osu! API client.
///
///  This does NOT automatically enable:
///  - Commands on the "osu" prefix
///  - Hooks. Hooks are completely opt-in.
///  
pub async fn setup(
    data: &mut TypeMap,
    prelude: youmubot_prelude::Env,
    announcers: &mut AnnouncerHandler,
) -> Result<OsuEnv> {
    // Databases
    let saved_users = OsuSavedUsers::new(prelude.sql.clone());
    let last_beatmaps = OsuLastBeatmap::new(prelude.sql.clone());
    let user_bests = OsuUserBests::new(prelude.sql.clone());

    // API client
    let osu_client = Arc::new(
        OsuHttpClient::new(
            std::env::var("OSU_API_CLIENT_ID")
                .expect("Please set OSU_API_CLIENT_ID as osu! api v2 client ID.")
                .parse()
                .expect("client_id should be u64"),
            std::env::var("OSU_API_CLIENT_SECRET")
                .expect("Please set OSU_API_CLIENT_SECRET as osu! api v2 client secret."),
        )
        .await
        .expect("osu! should be initialized"),
    );
    let oppai_cache = BeatmapCache::new(prelude.http.clone(), prelude.sql.clone());
    let beatmap_cache = BeatmapMetaCache::new(osu_client.clone(), prelude.sql.clone());

    // Announcer
    announcers.add(
        announcer::ANNOUNCER_KEY,
        announcer::Announcer::new(osu_client.clone()),
    );

    // Legacy data
    data.insert::<OsuLastBeatmap>(last_beatmaps.clone());
    data.insert::<OsuSavedUsers>(saved_users.clone());
    data.insert::<OsuUserBests>(user_bests.clone());
    data.insert::<OsuClient>(osu_client.clone());
    data.insert::<BeatmapCache>(oppai_cache.clone());
    data.insert::<BeatmapMetaCache>(beatmap_cache.clone());

    let env = OsuEnv {
        prelude,
        saved_users,
        last_beatmaps,
        user_bests,
        client: osu_client,
        oppai: oppai_cache,
        beatmaps: beatmap_cache,
    };

    data.insert::<OsuEnv>(env.clone());

    Ok(env)
}

#[group]
#[prefix = "osu"]
#[description = "osu! related commands."]
#[commands(
    std,
    taiko,
    catch,
    mania,
    save,
    forcesave,
    recent,
    last,
    check,
    top,
    server_rank,
    show_leaderboard,
    clean_cache
)]
#[default_command(std)]
struct Osu;

#[command]
#[aliases("osu", "osu!")]
#[description = "Receive information about an user in osu!std mode."]
#[usage = "[username or user_id = your saved username]"]
#[max_args(1)]
pub async fn std(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    get_user(ctx, &env, msg, args, Mode::Std).await
}

#[command]
#[aliases("osu!taiko")]
#[description = "Receive information about an user in osu!taiko mode."]
#[usage = "[username or user_id = your saved username]"]
#[max_args(1)]
pub async fn taiko(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    get_user(ctx, &env, msg, args, Mode::Taiko).await
}

#[command]
#[aliases("fruits", "osu!catch", "ctb")]
#[description = "Receive information about an user in osu!catch mode."]
#[usage = "[username or user_id = your saved username]"]
#[max_args(1)]
pub async fn catch(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    get_user(ctx, &env, msg, args, Mode::Catch).await
}

#[command]
#[aliases("osu!mania")]
#[description = "Receive information about an user in osu!mania mode."]
#[usage = "[username or user_id = your saved username]"]
#[max_args(1)]
pub async fn mania(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    get_user(ctx, &env, msg, args, Mode::Mania).await
}

pub(crate) struct BeatmapWithMode(pub Beatmap, pub Mode);

impl BeatmapWithMode {
    fn mode(&self) -> Mode {
        self.1
    }
}

impl AsRef<Beatmap> for BeatmapWithMode {
    fn as_ref(&self) -> &Beatmap {
        &self.0
    }
}

#[command]
#[description = "Save the given username as your username."]
#[usage = "[username or user_id]"]
#[num_args(1)]
pub async fn save(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    let osu_client = &env.client;

    let user = args.single::<String>()?;
    let u = match osu_client.user(UserID::from_string(user), |f| f).await? {
        Some(u) => u,
        None => {
            msg.reply(&ctx, "user not found...").await?;
            return Ok(());
        }
    };
    async fn find_score(client: &OsuHttpClient, u: &User) -> Result<Option<(Score, Mode)>> {
        for mode in &[Mode::Std, Mode::Taiko, Mode::Catch, Mode::Mania] {
            let scores = client
                .user_best(UserID::ID(u.id), |f| f.mode(*mode))
                .await?;
            if let Some(v) = scores.into_iter().choose(&mut rand::thread_rng()) {
                return Ok(Some((v, *mode)));
            }
        }
        Ok(None)
    }
    let (score, mode) = match find_score(osu_client, &u).await? {
        Some(v) => v,
        None => {
            msg.reply(
                &ctx,
                "No plays found in this account! Play something first...!",
            )
            .await?;
            return Ok(());
        }
    };

    async fn check(client: &OsuHttpClient, u: &User, map_id: u64) -> Result<bool> {
        Ok(client
            .user_recent(UserID::ID(u.id), |f| f.mode(Mode::Std).limit(1))
            .await?
            .into_iter()
            .take(1)
            .any(|s| s.beatmap_id == map_id))
    }

    let reply = msg.reply(
        &ctx,
        format!(
            "To set your osu username, please make your most recent play \
            be the following map: `/b/{}` in **{}** mode! \
        It does **not** have to be a pass, and **NF** can be used! \
        React to this message with 👌 within 5 minutes when you're done!",
            score.beatmap_id,
            mode.as_str_new_site()
        ),
    );
    let beatmap = osu_client
        .beatmaps(BeatmapRequestKind::Beatmap(score.beatmap_id), |f| {
            f.mode(mode, true)
        })
        .await?
        .into_iter()
        .next()
        .unwrap();
    let info = env
        .oppai
        .get_beatmap(beatmap.beatmap_id)
        .await?
        .get_possible_pp_with(mode, Mods::NOMOD)?;
    let mut reply = reply.await?;
    reply
        .edit(
            &ctx,
            EditMessage::new().embed(beatmap_embed(&beatmap, mode, Mods::NOMOD, info)),
        )
        .await?;
    let reaction = reply.react(&ctx, '👌').await?;
    let completed = loop {
        let emoji = reaction.emoji.clone();
        let user_reaction = collector::ReactionCollector::new(ctx)
            .message_id(reply.id)
            .author_id(msg.author.id)
            .filter(move |r| r.emoji == emoji)
            .timeout(std::time::Duration::from_secs(300))
            .next()
            .await;
        if let Some(ur) = user_reaction {
            if check(osu_client, &u, score.beatmap_id).await? {
                break true;
            }
            ur.delete(&ctx).await?;
        } else {
            break false;
        }
    };
    if !completed {
        reaction.delete(&ctx).await?;
        return Ok(());
    }

    let username = u.username.clone();
    add_user(msg.author.id, u, &env).await?;
    msg.reply(
        &ctx,
        MessageBuilder::new()
            .push("user has been set to ")
            .push_mono_safe(username)
            .build(),
    )
    .await?;
    Ok(())
}

#[command]
#[description = "Save the given username as someone's username."]
#[owners_only]
#[usage = "[ping user]/[username or user_id]"]
#[delimiters(" ")]
#[num_args(2)]
pub async fn forcesave(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();

    let osu_client = &env.client;

    let target = args.single::<UserId>()?.0;

    let username = args.quoted().trimmed().single::<String>()?;
    let user: Option<User> = osu_client
        .user(UserID::from_string(username.clone()), |f| f)
        .await?;
    match user {
        Some(u) => {
            add_user(target, u, &env).await?;
            msg.reply(
                &ctx,
                MessageBuilder::new()
                    .push("user has been set to ")
                    .push_mono_safe(username)
                    .build(),
            )
            .await?;
        }
        None => {
            msg.reply(&ctx, "user not found...").await?;
        }
    }
    Ok(())
}

async fn add_user(target: serenity::model::id::UserId, user: User, env: &OsuEnv) -> Result<()> {
    let u = OsuUser {
        user_id: target,
        username: user.username.into(),
        id: user.id,
        failures: 0,
        last_update: chrono::Utc::now(),
        pp: [None, None, None, None],
        std_weighted_map_length: None,
    };
    env.saved_users.new_user(u).await?;
    Ok(())
}

struct ModeArg(Mode);

impl FromStr for ModeArg {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ModeArg(match &s.to_lowercase()[..] {
            "osu" | "std" => Mode::Std,
            "taiko" | "osu!taiko" => Mode::Taiko,
            "ctb" | "fruits" | "catch" | "osu!ctb" | "osu!catch" => Mode::Catch,
            "osu!mania" | "mania" => Mode::Mania,
            _ => return Err(format!("Unknown mode {}", s)),
        }))
    }
}

async fn to_user_id_query(
    s: Option<UsernameArg>,
    env: &OsuEnv,
    msg: &Message,
) -> Result<UserID, Error> {
    let id = match s {
        Some(UsernameArg::Raw(s)) => return Ok(UserID::from_string(s)),
        Some(UsernameArg::Tagged(r)) => r,
        None => msg.author.id,
    };

    env.saved_users
        .by_user_id(id)
        .await?
        .map(|u| UserID::ID(u.id))
        .ok_or_else(|| Error::msg("No saved account found"))
}

enum Nth {
    All,
    Nth(u8),
}

impl FromStr for Nth {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "--all" || s == "-a" || s == "##" {
            Ok(Nth::All)
        } else if !s.starts_with('#') {
            Err(Error::msg("Not an order"))
        } else {
            let v = s.split_at("#".len()).1.parse()?;
            Ok(Nth::Nth(v))
        }
    }
}

#[command]
#[aliases("rs", "rc", "r")]
#[description = "Gets an user's recent play"]
#[usage = "#[the nth recent play = --all] / [style (table or grid) = --table] / [mode (std, taiko, mania, catch) = std] / [username / user id = your saved id]"]
#[example = "#1 / taiko / natsukagami"]
#[delimiters("/", " ")]
#[max_args(4)]
pub async fn recent(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();

    let nth = args.single::<Nth>().unwrap_or(Nth::All);
    let style = args.single::<ScoreListStyle>().unwrap_or_default();
    let mode = args.single::<ModeArg>().unwrap_or(ModeArg(Mode::Std)).0;
    let user = to_user_id_query(
        args.quoted().trimmed().single::<UsernameArg>().ok(),
        &env,
        msg,
    )
    .await?;

    let osu_client = &env.client;

    let user = osu_client
        .user(user, |f| f.mode(mode))
        .await?
        .ok_or_else(|| Error::msg("User not found"))?;
    match nth {
        Nth::Nth(nth) => {
            let recent_play = osu_client
                .user_recent(UserID::ID(user.id), |f| f.mode(mode).limit(nth))
                .await?
                .into_iter()
                .last()
                .ok_or_else(|| Error::msg("No such play"))?;
            let beatmap = env
                .beatmaps
                .get_beatmap(recent_play.beatmap_id, mode)
                .await?;
            let content = env.oppai.get_beatmap(beatmap.beatmap_id).await?;
            let beatmap_mode = BeatmapWithMode(beatmap, mode);

            msg.channel_id
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .content("Here is the play that you requested".to_string())
                        .embed(score_embed(&recent_play, &beatmap_mode, &content, &user).build())
                        .reference_message(msg),
                )
                .await?;

            // Save the beatmap...
            cache::save_beatmap(&env, msg.channel_id, &beatmap_mode).await?;
        }
        Nth::All => {
            let plays = osu_client
                .user_recent(UserID::ID(user.id), |f| f.mode(mode).limit(50))
                .await?;
            style.display_scores(plays, mode, ctx, msg).await?;
        }
    }
    Ok(())
}

/// Get beatmapset.
struct OptBeatmapSet;

impl FromStr for OptBeatmapSet {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "--set" | "-s" | "--beatmapset" => Ok(Self),
            _ => Err(Error::msg("not opt beatmapset")),
        }
    }
}

/// Load the mentioned beatmap from the given message.
pub(crate) async fn load_beatmap(
    env: &OsuEnv,
    msg: &Message,
) -> Option<(BeatmapWithMode, Option<Mods>)> {
    if let Some(replied) = &msg.referenced_message {
        // Try to look for a mention of the replied message.
        let beatmap_id = SHORT_LINK_REGEX.captures(&replied.content).or_else(|| {
            replied.embeds.iter().find_map(|e| {
                e.description
                    .as_ref()
                    .and_then(|v| SHORT_LINK_REGEX.captures(v))
                    .or_else(|| {
                        e.fields
                            .iter()
                            .find_map(|f| SHORT_LINK_REGEX.captures(&f.value))
                    })
            })
        });
        if let Some(caps) = beatmap_id {
            let id: u64 = caps.name("id").unwrap().as_str().parse().unwrap();
            let mode = caps
                .name("mode")
                .and_then(|m| Mode::parse_from_new_site(m.as_str()));
            let mods = caps
                .name("mods")
                .and_then(|m| m.as_str().parse::<Mods>().ok());
            let osu_client = &env.client;
            let bms = osu_client
                .beatmaps(BeatmapRequestKind::Beatmap(id), |f| f.maybe_mode(mode))
                .await
                .ok()
                .and_then(|v| v.into_iter().next());
            if let Some(beatmap) = bms {
                let bm_mode = beatmap.mode;
                let bm = BeatmapWithMode(beatmap, mode.unwrap_or(bm_mode));
                // Store the beatmap in history
                cache::save_beatmap(&env, msg.channel_id, &bm)
                    .await
                    .pls_ok();

                return Some((bm, mods));
            }
        }
    }

    let b = cache::get_beatmap(&env, msg.channel_id)
        .await
        .ok()
        .flatten();
    b.map(|b| (b, None))
}

#[command]
#[aliases("map")]
#[description = "Show information from the last queried beatmap."]
#[usage = "[--set/-s/--beatmapset] / [mods = no mod]"]
#[delimiters(" ")]
#[max_args(2)]
pub async fn last(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();

    let b = load_beatmap(&env, msg).await;
    let beatmapset = args.find::<OptBeatmapSet>().is_ok();

    match b {
        Some((BeatmapWithMode(b, m), mods_def)) => {
            let mods = args.find::<Mods>().ok().or(mods_def).unwrap_or(Mods::NOMOD);
            if beatmapset {
                let beatmapset = env.beatmaps.get_beatmapset(b.beatmapset_id).await?;
                display::display_beatmapset(
                    ctx,
                    beatmapset,
                    None,
                    Some(mods),
                    msg,
                    "Here is the beatmapset you requested!",
                )
                .await?;
                return Ok(());
            }
            let info = env
                .oppai
                .get_beatmap(b.beatmap_id)
                .await?
                .get_possible_pp_with(m, mods)?;
            msg.channel_id
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .content("Here is the beatmap you requested!")
                        .embed(beatmap_embed(&b, m, mods, info))
                        .reference_message(msg),
                )
                .await?;
        }
        None => {
            msg.reply(&ctx, "No beatmap was queried on this channel.")
                .await?;
        }
    }

    Ok(())
}

#[command]
#[aliases("c", "chk")]
#[usage = "[style (table or grid) = --table] / [username or tag = yourself] / [mods to filter]"]
#[description = "Check your own or someone else's best record on the last beatmap. Also stores the result if possible."]
#[max_args(3)]
pub async fn check(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    let bm = load_beatmap(&env, msg).await;

    let bm = match bm {
        Some((bm, _)) => bm,
        None => {
            msg.reply(&ctx, "No beatmap queried on this channel.")
                .await?;
            return Ok(());
        }
    };

    let mods = args.find::<Mods>().ok().unwrap_or_default();
    let b = &bm.0;
    let m = bm.1;
    let style = args
        .single::<ScoreListStyle>()
        .unwrap_or(ScoreListStyle::Grid);
    let username_arg = args.single::<UsernameArg>().ok();
    let user_id = match username_arg.as_ref() {
        Some(UsernameArg::Tagged(v)) => Some(*v),
        None => Some(msg.author.id),
        _ => None,
    };
    let user = to_user_id_query(username_arg, &env, msg).await?;

    let osu_client = env.client;

    let user = osu_client
        .user(user, |f| f)
        .await?
        .ok_or_else(|| Error::msg("User not found"))?;
    let mut scores = osu_client
        .scores(b.beatmap_id, |f| f.user(UserID::ID(user.id)).mode(m))
        .await?
        .into_iter()
        .filter(|s| s.mods.contains(mods))
        .collect::<Vec<_>>();
    scores.sort_by(|a, b| {
        b.pp.unwrap_or(-1.0)
            .partial_cmp(&a.pp.unwrap_or(-1.0))
            .unwrap()
    });

    if scores.is_empty() {
        msg.reply(&ctx, "No scores found").await?;
        return Ok(());
    }

    if let Some(user_id) = user_id {
        // Save to database
        env.user_bests
            .save(user_id, m, scores.clone())
            .await
            .pls_ok();
    }

    style.display_scores(scores, m, ctx, msg).await?;

    Ok(())
}

#[command]
#[aliases("t")]
#[description = "Get the n-th top record of an user."]
#[usage = "#[n-th = --all] / [style (table or grid) = --table] / [mode (std, taiko, catch, mania)] = std / [username or user_id = your saved user id]"]
#[example = "#2 / taiko / natsukagami"]
#[max_args(4)]
pub async fn top(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    let nth = args.single::<Nth>().unwrap_or(Nth::All);
    let style = args.single::<ScoreListStyle>().unwrap_or_default();
    let mode = args
        .single::<ModeArg>()
        .map(|ModeArg(t)| t)
        .unwrap_or(Mode::Std);

    let user = to_user_id_query(args.single::<UsernameArg>().ok(), &env, msg).await?;
    let osu_client = &env.client;
    let user = osu_client
        .user(user, |f| f.mode(mode))
        .await?
        .ok_or_else(|| Error::msg("User not found"))?;

    match nth {
        Nth::Nth(nth) => {
            let top_play = osu_client
                .user_best(UserID::ID(user.id), |f| f.mode(mode).limit(nth))
                .await?;

            let rank = top_play.len() as u8;

            let top_play = top_play
                .into_iter()
                .last()
                .ok_or_else(|| Error::msg("No such play"))?;
            let beatmap = env.beatmaps.get_beatmap(top_play.beatmap_id, mode).await?;
            let content = env.oppai.get_beatmap(beatmap.beatmap_id).await?;
            let beatmap = BeatmapWithMode(beatmap, mode);

            msg.channel_id
                .send_message(&ctx, {
                    CreateMessage::new()
                        .content(format!(
                            "{}: here is the play that you requested",
                            msg.author
                        ))
                        .embed(
                            score_embed(&top_play, &beatmap, &content, &user)
                                .top_record(rank)
                                .build(),
                        )
                })
                .await?;

            // Save the beatmap...
            cache::save_beatmap(&env, msg.channel_id, &beatmap).await?;
        }
        Nth::All => {
            let plays = osu_client
                .user_best(UserID::ID(user.id), |f| f.mode(mode).limit(100))
                .await?;
            style.display_scores(plays, mode, ctx, msg).await?;
        }
    }
    Ok(())
}

#[command("cleancache")]
#[owners_only]
#[description = "Clean the beatmap cache."]
#[usage = "[--oppai to clear oppai cache as well]"]
#[max_args(1)]
pub async fn clean_cache(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let env = ctx.data.read().await.get::<OsuEnv>().unwrap().clone();
    env.beatmaps.clear().await?;

    if args.remains() == Some("--oppai") {
        env.oppai.clear().await?;
    }
    msg.reply_ping(ctx, "Beatmap cache cleared!").await?;
    Ok(())
}

async fn get_user(
    ctx: &Context,
    env: &OsuEnv,
    msg: &Message,
    mut args: Args,
    mode: Mode,
) -> CommandResult {
    let user = to_user_id_query(args.single::<UsernameArg>().ok(), &env, msg).await?;
    let osu_client = &env.client;
    let meta_cache = &env.beatmaps;
    let user = osu_client.user(user, |f| f.mode(mode)).await?;

    match user {
        Some(u) => {
            let bests = osu_client
                .user_best(UserID::ID(u.id), |f| f.limit(100).mode(mode))
                .await?;
            let map_length = calculate_weighted_map_length(&bests, meta_cache, mode).await?;
            let best = match bests.into_iter().next() {
                Some(m) => {
                    let beatmap = meta_cache.get_beatmap(m.beatmap_id, mode).await?;
                    let info = env
                        .oppai
                        .get_beatmap(m.beatmap_id)
                        .await?
                        .get_info_with(mode, m.mods)?;
                    Some((m, BeatmapWithMode(beatmap, mode), info))
                }
                None => None,
            };
            msg.channel_id
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .content(format!(
                            "{}: here is the user that you requested",
                            msg.author
                        ))
                        .embed(user_embed(u, map_length, best)),
                )
                .await?;
        }
        None => {
            msg.reply(&ctx, "🔍 user not found!").await?;
        }
    };
    Ok(())
}

pub(in crate::discord) async fn calculate_weighted_map_length(
    from_scores: impl IntoIterator<Item = &Score>,
    cache: &BeatmapMetaCache,
    mode: Mode,
) -> Result<f64> {
    from_scores
        .into_iter()
        .enumerate()
        .map(|(i, s)| async move {
            let beatmap = cache.get_beatmap(s.beatmap_id, mode).await?;
            const SCALING_FACTOR: f64 = 0.975;
            Ok(beatmap
                .difficulty
                .apply_mods(s.mods, 0.0 /* dont care */)
                .drain_length
                .as_secs_f64()
                * (SCALING_FACTOR.powi(i as i32)))
        })
        .collect::<FuturesUnordered<_>>()
        .try_fold(0.0, |a, b| future::ready(Ok(a + b)))
        .await
}
