use crate::http::*;
use futures::future::try_join_all;
use minnie_errors::*;
use minnie_model::channel::*;
use minnie_model::guild::*;
use minnie_model::types::*;
use std::borrow::Cow;

/// Performs operations relating to guilds.
///
/// Instances can be obtained by calling [`DiscordContext::guild`](`crate::DiscordContext::guild`),
/// [`Guild::ops`], or [`PartialGuild::ops`].
#[derive(Debug, Clone)]
pub struct GuildOps<'a> {
    pub(crate) id: GuildId,
    pub(crate) raw: Routes<'a>,
}
impl <'a> GuildOps<'a> {
    /// Performs operations related to a guild member.
    pub fn member(self, id: impl Into<UserId>) -> MemberOps<'a> {
        MemberOps { guild_id: self.id, user_id: id.into(), raw: self.raw }
    }

    // TODO: Create Guilds

    /// Modifies the guild's settings.
    ///
    /// For information on what properties can be set, see the methods of [`ModifyGuildFut`].
    pub fn modify(self) -> ModifyGuildFut<'a> {
        ModifyGuildFut::new(self)
    }

    /// Deletes this guild.
    pub async fn delete(self) -> Result<()> {
        self.raw.delete_guild(self.id).await
    }

    /// Gets a list of channels in this guild.
    pub async fn get_channels(self) -> Result<Vec<Channel>> {
        self.raw.get_guild_channels(self.id).await
    }

    // TODO: Create Channel
    // TODO: Modify Guild Channel Position
    // TODO: List Guild Members
    // TODO: Add Guild Member
    // TODO: Get Guild Bans
    // TODO: Get Guild Ban

    /// Changes the bot's username on the guild.
    pub async fn change_nick(self, nick: impl AsRef<str>) -> Result<()> {
        self.raw.modify_current_user_nick(self.id, nick.as_ref()).await
    }

    /// Retrieves a list of roles in this guild.
    pub async fn get_roles(self) -> Result<Vec<Role>> {
        self.raw.get_guild_roles(self.id).await
    }

    // TODO: Create Guild Role
    // TODO: Modify Guild Role Positions
    // TODO: Modify Guild Role
    // TODO: Delete Guild Role
    // TODO: Begin Guild Prune

    /// Retrieves a list of voice regions available to this guild.
    pub async fn get_voice_regions(self) -> Result<Vec<VoiceRegion>> {
        self.raw.get_guild_voice_regions(self.id).await
    }

    /// Retrieves a list of invites to this guild.
    pub async fn get_invites(self) -> Result<Vec<InviteWithMetadata>> {
        self.raw.get_guild_invites(self.id).await
    }

    /// Retrieves the embed settings for this guild.
    pub async fn get_embed_settings(self) -> Result<GuildEmbedSettings> {
        self.raw.get_guild_embed(self.id).await
    }

    // TODO: Modify Guild Embed

    /// Returns a link to this guild's vanity invite if one exists.
    pub async fn get_vanity_url(self) -> Result<Option<String>> {
        let result = self.raw.get_guild_vanity_url(self.id).await?;
        Ok(result.code.map(|x| format!("https://discord.gg/{}", x)))
    }

    routes_wrapper!(self, &mut self.raw);
}

/// Performs operations relating to guild members.
///
/// Instances can be obtained by calling [`GuildOps::member`] or
/// [`DiscordCrate::member`](`crate::DiscordContext::member`).
#[derive(Debug, Clone)]
pub struct MemberOps<'a> {
    pub(crate) guild_id: GuildId,
    pub(crate) user_id: UserId,
    pub(crate) raw: Routes<'a>,
}
impl <'a> MemberOps<'a> {
    /// Retrieves information relating to this member.
    pub async fn get(self) -> Result<Member> {
        self.raw.get_guild_member(self.guild_id, self.user_id).await
    }

    /// Modifies the user's permissions, nickname and related settings.
    ///
    /// For information on what properties can be set, see the methods of [`ModifyGuildMemberFut`].
    pub fn modify(self) -> ModifyGuildMemberFut<'a> {
        ModifyGuildMemberFut::new(self)
    }

    /// Adds a role to this member.
    pub async fn add_role(self, role: impl Into<RoleId>) -> Result<()> {
        self.raw.add_guild_member_role(self.guild_id, self.user_id, role.into()).await
    }

    /// Adds multiple roles to this member.
    ///
    /// This will make an API call for each role in the list. The API calls will be
    /// dispatched simultaneously.
    pub async fn add_roles(
        self, roles: impl IntoIterator<Item = impl Into<RoleId>>,
    ) -> Result<()> {
        let mut role_futs = Vec::new();
        for role in roles {
            role_futs.push(self.clone().add_role(role));
        }
        try_join_all(role_futs).await?;
        Ok(())
    }

    /// Removes a role from this member.
    pub async fn remove_role(self, role: impl Into<RoleId>) -> Result<()> {
        self.raw.remove_guild_member_role(self.guild_id, self.user_id, role.into()).await
    }

    /// Removes multiple roles to this member.
    ///
    /// This will make an API call for each role in the list. The API calls will be
    /// dispatched simultaneously.
    pub async fn remove_roles(
        self, roles: impl IntoIterator<Item = impl Into<RoleId>>,
    ) -> Result<()> {
        let mut role_futs = Vec::new();
        for role in roles {
            role_futs.push(self.clone().remove_role(role));
        }
        try_join_all(role_futs).await?;
        Ok(())
    }

    /// Kicks this member from the guild.
    pub async fn kick(self) -> Result<()> {
        self.raw.remove_guild_member(self.guild_id, self.user_id).await
    }

    // TODO: Ban

    /// Unbans a user from the guild.
    pub async fn unban(self) -> Result<()> {
        self.raw.remove_guild_ban(self.guild_id, self.user_id).await
    }

    routes_wrapper!(self, &mut self.raw);
}

fn check_is_image(image: &ImageData) -> Result<()> {
    match image.format() {
        ImageFormat::Png | ImageFormat::Jpeg => { }
        _ => bail!(InvalidInput, "Image must be PNG or JPEG."),
    }
    Ok(())
}
fn check_is_anim_image(image: &ImageData) -> Result<()> {
    match image.format() {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif => { }
        _ => bail!(InvalidInput, "Image must be GIF, PNG or JPEG."),
    }
    Ok(())
}

fut_builder! {
    ('a, modify_guild_mod, GuildOps, self)

    /// A future for modifying settings of a guild.
    ///
    /// Instances can be obtained via [`GuildOps::modify`].
    struct ModifyGuildFut {
        params: ModifyGuildParams<'a>,
    }
    into_async!(|ops, data| -> Result<Guild> {
        if let Some(img) = &data.params.icon {
            check_is_anim_image(&img)?;
        }
        if let Some(img) = &data.params.splash {
            check_is_image(&img)?;
        }
        if let Some(img) = &data.params.banner {
            check_is_image(&img)?;
        }
        ops.raw.modify_guild(ops.id, data.params).await
    });

    /// Sets the name of this guild.
    pub fn name(&mut self, name: impl Into<Cow<'a, str>>) {
        self.params.name = Some(name.into());
    }

    /// Sets the voice region this guild's voice channels use.
    pub fn voice_region(&mut self, voice_region: impl Into<Cow<'a, str>>) {
        self.params.region = Some(voice_region.into());
    }

    /// Sets the level of verification required for users to speak in this guild.
    pub fn verification_level(&mut self, verification_level: VerificationLevel) {
        self.params.verification_level = Some(verification_level);
    }

    /// Sets the default notification level for this guild.
    pub fn notification_level(&mut self, notification_level: NotificationLevel) {
        self.params.default_message_notifications = Some(notification_level);
    }

    /// Sets the strictness of the default explicit content filter on this guild.
    pub fn content_filter_level(&mut self, content_filter_level: ExplicitContentFilterLevel) {
        self.params.explicit_content_filter = Some(content_filter_level);
    }

    /// Sets the AFK voice channel.
    pub fn afk_channel(&mut self, channel: impl Into<ChannelId>) {
        self.params.afk_channel_id = Some(channel.into())
    }

    /// Sets the number of seconds a user must be idle in a voice channel after which they are
    /// considered AFK, and will be automatically moved into the AFK channel.
    pub fn afk_timeout(&mut self, timeout: u32) {
        self.params.afk_timeout = Some(timeout);
    }

    /// Sets the icon of the guild.
    pub fn icon(&mut self, icon: ImageData<'a>) {
        self.params.icon = Some(icon);
    }

    /// Transfers ownership of the guild to another user. The bot must own the guild.
    pub fn transfer_ownership(&mut self, id: impl Into<UserId>) {
        self.params.owner_id = Some(id.into());
    }

    /// Sets the invite splash of the guild. The guild must have the feature enabled.
    pub fn invite_splash(&mut self, splash: ImageData<'a>) {
        self.params.splash = Some(splash.into());
    }

    /// Sets the banner of the guild. The guild must have the feature enabled.
    pub fn banner(&mut self, banner: ImageData<'a>) {
        self.params.banner = Some(banner.into());
    }

    /// Sets the system channel where Discord automatically posts user join, part and server
    /// boost messages.
    pub fn system_channel(&mut self, id: impl Into<ChannelId>) {
        self.params.system_channel_id = Some(id.into());
    }
}

fut_builder! {
    ('a, modify_guild_member_mod, MemberOps, self)

    /// A future for modifying a user's permissions in a guild.
    ///
    /// Instances can be obtained via [`MemberOps::modify`].
    struct ModifyGuildMemberFut {
        params: ModifyGuildMemberParams<'a>,
    }
    into_async!(|ops, data| -> Result<()> {
        ops.raw.modify_guild_member(ops.guild_id, ops.user_id, data.params).await
    });

    /// Changes the user's nickname.
    pub fn nick(&mut self, nick: impl Into<Cow<'a, str>>) {
        self.params.nick = Some(nick.into());
    }

    /// Sets the user's roles.
    ///
    /// This is not recommended as there is the possibility of a race condition between your bot
    /// and another bot, especially on events such as user join.
    ///
    /// See [`MemberOps::add_roles`] and [`MemberOps::remove_roles`] for a safer alternative.
    ///
    /// This API call should generally only be used for custom bots on servers with no other bots
    /// that manage roles, or if absolutely required for performance reasons.
    pub fn roles(&mut self, roles: impl Into<Cow<'a, [RoleId]>>) {
        self.params.roles = Some(roles.into());
    }

    /// Mutes the user on voice channels.
    pub fn mute_voice(&mut self) {
        self.params.mute = Some(true);
    }

    /// Unmutes the user on voice channels.
    pub fn unmute_voice(&mut self) {
        self.params.mute = Some(false);
    }

    /// Sets whether the user is muted on voice channels.
    pub fn voice_muted(&mut self, muted: bool) {
        self.params.mute = Some(muted);
    }

    /// Deafens the user on voice channels.
    pub fn deafen(&mut self) {
        self.params.deaf = Some(true);
    }

    /// Undeafens the user on voice channels.
    pub fn undeafen(&mut self) {
        self.params.deaf = Some(false);
    }

    /// Sets whether the user is deafened on voice channels.
    pub fn deafened(&mut self, deafened: bool) {
        self.params.deaf = Some(deafened);
    }

    /// Moves the user to a given voice channel.
    pub fn move_to(&mut self, channel: impl Into<ChannelId>) {
        self.params.channel_id = Some(Some(channel.into()));
    }

    /// Disconnects the user from their voice channel.
    pub fn disconnect_voice(&mut self) {
        self.params.channel_id = Some(None);
    }
}

