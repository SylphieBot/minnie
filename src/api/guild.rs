use crate::errors::*;
use crate::http::*;
use crate::model::channel::*;
use crate::model::guild::*;
use crate::model::types::*;
use futures::future::try_join_all;

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
    // TODO: Modify Guild

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

    // TODO: Modify Guild Member

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

    pub async fn unban(self) -> Result<()> {
        self.raw.remove_guild_ban(self.guild_id, self.user_id).await
    }

    routes_wrapper!(self, &mut self.raw);
}
