use crate::errors::*;
use crate::http::*;
use crate::model::channel::*;
use crate::model::message::*;
use crate::model::types::*;
use crate::model::user::*;
use enumset::*;
use futures::future::try_join_all;
use std::borrow::Cow;

/// Performs operations relating to a Discord channel.
///
/// Instances can be obtained by calling
/// [`DiscordContext::channel`](`crate::DiscordContext::channel`),
/// [`Channel::ops`], or [`PartialChannel::ops`].
#[derive(Debug, Clone)]
pub struct ChannelOps<'a> {
    pub(crate) id: ChannelId,
    pub(crate) raw: Routes<'a>,
}
impl <'a> ChannelOps<'a> {
    /// Performs operations relating to a message.
    pub async fn message(self, id: impl Into<MessageId>) -> MessageOps<'a> {
        MessageOps { channel_id: self.id, message_id: id.into(), raw: self.raw }
    }

    /// Retrieves information relating to the channel.
    pub async fn get(self) -> Result<Channel> {
        self.raw.get_channel(self.id).await
    }

    /// Modifies the channel's setting, such as its name or topic.
    ///
    /// For information on what properties can be set, see the methods of [`ModifyChannelFut`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use minnie::DiscordContext;
    /// # use minnie::Result;
    /// # use minnie::model::types::ChannelId;
    /// async fn set_channel_name(ctx: DiscordContext, id: ChannelId) -> Result<()> {
    ///     ctx.channel(id).edit().name("foo").topic("bar").await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn modify(self) -> ModifyChannelFut<'a> {
        ModifyChannelFut::new(self)
    }

    /// Deletes the channel.
    pub async fn delete(self) -> Result<Channel> {
        self.raw.delete_channel(self.id).await
    }

    /// Retrieves the channel history.
    ///
    /// By default, returns the latest 50 messages to the channel. For more information on other
    /// options for this API call, see the methods of [`GetMessageHistoryFut`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use minnie::DiscordContext;
    /// # use minnie::Result;
    /// # use minnie::model::types::ChannelId;
    /// async fn print_messages(ctx: DiscordContext, id: ChannelId) -> Result<()> {
    ///     println!("{:?}", ctx.channel(id).get_message_history().await?);
    ///     println!("{:?}", ctx.channel(id).get_message_history().limit(100).await?);
    ///     Ok(())
    /// }
    /// ```
    pub fn get_message_history(self) -> GetMessageHistoryFut<'a> {
        GetMessageHistoryFut::new(self)
    }

    /// Posts a message to this channel.
    ///
    /// Use the [`content`](`PostFut::content`) and [`embed`](`PostFut::embed`) methods of the
    /// returned future to set the post contents. At least one of `content`, `embed`, or `file`
    /// must be called or an error will be returned.
    ///
    /// For more information on other options for this API call, see the methods of [`PostFut`].
    pub fn post(self) -> PostFut<'a> {
        PostFut::new(self)
    }

    /// Deletes a list of messages.
    ///
    /// This will make an API call for each 100 messages in the list. The API calls will be
    /// dispatched simultaneously.
    pub async fn delete_messages(self, messages: impl Into<Cow<'a, [MessageId]>>) -> Result<()> {
        let messages = messages.into();
        if messages.len() == 1 {
            self.raw.delete_message(self.id, messages[0]).await?;
        } else if messages.len() <= 100 {
            self.raw.bulk_delete_message(self.id, &messages).await?;
        } else {
            let mut delete_futs = Vec::new();
            for chunk in messages.chunks(100) {
                delete_futs.push(self.raw.clone().bulk_delete_message(self.id, chunk));
            }
            try_join_all(delete_futs).await?;
        }
        Ok(())
    }

    /// Completely overwrites the permission overwrite for a given user or role.
    ///
    /// If the given `allow` and `deny` sets are both empty, the overwrite is instead deleted.
    /// Explicitly allowed permissions have precedence over explicitly denied permissions.
    pub async fn set_permissions(
        self, overwrite: impl Into<PermissionOverwriteId>,
        allow: impl Into<EnumSet<Permission>>, deny: impl Into<EnumSet<Permission>>,
    ) -> Result<()> {
        let allow = allow.into();
        let mut deny = deny.into();
        deny -= allow;
        if allow.is_empty() && deny.is_empty() {
            self.raw.delete_channel_permission(self.id, overwrite.into()).await
        } else {
            self.raw.edit_channel_permissions(
                self.id, overwrite.into(),
                EditChannelPermissionsParams::new(allow.into(), deny.into())
            ).await
        }
    }

    /// Retrieves a list of invites to this channel.
    pub async fn get_invites(self) -> Result<Vec<InviteWithMetadata>> {
        self.raw.get_channel_invites(self.id).await
    }

    /// Creates an invite to this channel.
    ///
    /// By default, this creates an invite valid for 24 hours, and reuses old invite codes. For
    /// more information on other options for this API call, see the methods of [`InviteFut`].
    pub fn invite(self) -> InviteFut<'a> {
        InviteFut::new(self)
    }

    /// Clears all permission overwrites for a given user or role.
    pub async fn clear_permissions(
        self, overwrite: impl Into<PermissionOverwriteId>,
    ) -> Result<()> {
        self.raw.delete_channel_permission(self.id, overwrite.into()).await
    }

    /// Triggers the typing indicator.
    pub async fn typing(self) -> Result<()> {
        self.raw.trigger_typing_indicator(self.id).await
    }

    /// Retrieves a list of messages pinned to this channel.
    pub async fn get_pinned_messages(self) -> Result<Vec<Message>> {
        self.raw.get_pinned_messages(self.id).await
    }

    routes_wrapper!(self, &mut self.raw);
}

/// Performs operations relating to a message.
///
/// Instances can be obtained by calling
/// [`DiscordContext::message`](`crate::DiscordContext::message`),
/// [`ChannelOps::message`] or [`Message::ops`].
#[derive(Debug, Clone)]
pub struct MessageOps<'a> {
    pub(crate) channel_id: ChannelId,
    pub(crate) message_id: MessageId,
    pub(crate) raw: Routes<'a>,
}
impl <'a> MessageOps<'a> {
    /// Retrieves information relating to this message.
    pub async fn get(self) -> Result<Message> {
        self.raw.get_channel_message(self.channel_id, self.message_id).await
    }

    /// Reacts to this message.
    pub async fn react(self, emoji: &EmojiRef) -> Result<()> {
        self.raw.create_reaction(self.channel_id, self.message_id, emoji).await
    }

    /// Removes the bot's reaction to this message.
    pub async fn delete_own_reaction(self, emoji: &EmojiRef) -> Result<()> {
        self.raw.delete_own_reaction(self.channel_id, self.message_id, emoji).await
    }

    /// Removes another user's reaction to this message.
    pub async fn delete_reaction(self, emoji: &EmojiRef, user: UserId) -> Result<()> {
        self.raw.delete_user_reaction(self.channel_id, self.message_id, emoji, user).await
    }

    /// Retrieves a list of users who reacted with a particular emoji to this message.
    ///
    /// By default, this returns the first 25 users in the list. For more information on other
    /// options for this API call, see the methods of [`EmojiReactionsFut`].
    pub async fn emoji_reactions(self, emoji: &'a EmojiRef) -> EmojiReactionsFut<'a> {
        EmojiReactionsFut::new(self, emoji)
    }

    /// Deletes all reactions from a message.
    pub async fn clear_reactions(self) -> Result<()> {
        self.raw.delete_all_reactions(self.channel_id, self.message_id).await
    }

    /// Edits this message.
    /// 
    /// This has similar parameters to posting messages, but only [`content`](`EditFut::content`)
    /// and [`embed`](`EditFut::embed`) are supported.
    pub fn edit(self) -> EditFut<'a> {
        EditFut::new(self)
    }

    /// Deletes this message.
    pub async fn delete(self) -> Result<()> {
        self.raw.delete_message(self.channel_id, self.message_id).await
    }

    /// Pins this message to its channel.
    pub async fn pin(self) -> Result<()> {
        self.raw.add_pinned_channel_message(self.channel_id, self.message_id).await
    }

    /// Unpins this message from its channel.
    pub async fn unpin(self) -> Result<()> {
        self.raw.delete_pinned_channel_message(self.channel_id, self.message_id).await
    }

    routes_wrapper!(self, &mut self.raw);
}

fut_builder! {
    ('a, modify_channel_mod, ChannelOps, self)

    /// A future for operations that modify Discord channels.
    ///
    /// Instances can be obtained via [`ChannelOps::modify`].
    struct ModifyChannelFut {
        params: ModifyChannelParams<'a>,
    }
    into_async!(|ops, data| -> Result<Channel> {
        ops.raw.modify_channel(ops.id, data.params).await
    });

    /// Sets the position of this channel.
    pub fn name(&mut self, name: impl Into<Cow<'a, str>>) {
        self.params.name = Some(name.into());
    }

    /// Sets the position of this channel.
    pub fn position(&mut self, position: u32) {
        self.params.position = Some(position);
    }

    /// Sets the topic of this channel.
    ///
    /// Only available for text channels.
    pub fn topic(&mut self, topic: impl Into<Cow<'a, str>>) {
        self.params.topic = Some(topic.into());
    }

    /// Sets whether this channel is considered NSFW.
    ///
    /// Only available for text channels.
    pub fn nsfw(&mut self, nsfw: bool) {
        self.params.nsfw = Some(nsfw);
    }

    /// Sets the number of seconds users in this channel must wait before posting another message.
    /// A value of 0 represents no slow mode.
    ///
    /// Currently limited to 0-21600 seconds.
    ///
    /// Only available for text channels.
    pub fn slow_mode(&mut self, rate_limit: u32) {
        self.params.rate_limit_per_user = Some(rate_limit);
    }

    /// Sets the bitrate of this channel.
    ///
    /// Current limited to 8000-96000 bits/second, The limit is increased to 128000 for
    /// VIP servers.
    ///
    /// Only available for voice channels.
    pub fn bitrate(&mut self, bitrate: u32) {
        self.params.bitrate = Some(bitrate);
    }

    /// Sets the user limit for this channel.
    ///
    /// Only available for voice channels.
    pub fn user_limit(&mut self, limit: u32) {
        self.params.bitrate = Some(limit);
    }

    /// Sets the permission overwrites for this channel.
    pub fn permission_overwrites(&mut self, data: impl Into<Cow<'a, [PermissionOverwrite]>>) {
        self.params.permission_overwrites = Some(data.into());
    }

    /// Sets the category this channel is in.
    pub fn category(&mut self, parent: Option<impl Into<ChannelId>>) {
        self.params.parent_id = Some(parent.map(Into::into));
    }
}

fut_builder! {
    ('a, get_message_history_mod, ChannelOps, self)

    /// A future for a channel's message history.
    ///
    /// Instances can be obtained via [`ChannelOps::get_message_history`].
    struct GetMessageHistoryFut {
        params: GetChannelMessagesParams<'a>,
    }
    into_async!(|ops, data| -> Result<Vec<Message>> {
        if (data.params.around.is_some() && data.params.after.is_some()) ||
           (data.params.before.is_some() && data.params.after.is_some()) ||
           (data.params.around.is_some() && data.params.before.is_some())
        {
            bail!(InvalidInput, "Can only set one of `around`, `before`, and `after.");
        }

        ops.raw.get_channel_messages(ops.id, data.params).await
    });

    /// Gets messages around the message ID.
    ///
    /// Mutually exclusive with `before` and `after`.
    pub fn around(&mut self, id: impl Into<MessageId>) {
        self.params.around = Some(id.into());
    }

    /// Gets messages before the message ID.
    ///
    /// Mutually exclusive with `around` and `after`.
    pub fn before(&mut self, id: impl Into<MessageId>) {
        self.params.before = Some(id.into());
    }

    /// Gets messages after the message ID.
    ///
    /// Mutually exclusive with `around` and `before`.
    pub fn after(&mut self, id: impl Into<MessageId>) {
        self.params.after = Some(id.into());
    }

    /// Sets the number of messages to return.
    ///
    /// Currently limited to 1-100 messages. Defaults to 50 messages.
    pub fn limit(&mut self, limit: u32) {
        self.params.limit = Some(limit);
    }
}

fut_builder! {
    ('a, post_fut_mod, ChannelOps, self)

    /// A future for posting a new message.
    ///
    /// Instances can be obtained via [`ChannelOps::post`].
    struct PostFut {
        params: CreateMessageParams<'a>,
        files: Vec<CreateMessageFile<'a>>,
    }
    into_async!(|ops, data| -> Result<Message> {
        if data.files.is_empty() && data.params.content.is_none() && data.params.embed.is_none() {
            bail!(InvalidInput, "At least one of `content` or `embed` must be set, or a file must \
                                 be uploaded.");
        }
        ops.raw.create_message(ops.id, data.params, data.files).await
    });

    /// Sets the content of the post.
    pub fn content(&mut self, content: impl Into<Cow<'a, str>>) {
        self.params.content = Some(content.into());
    }

    /// Sets the nonce for this post.
    ///
    /// The nonce will be present in the event the messages triggers in the gateway, and can be
    /// used to confirm that the message has been successfully sent.
    pub fn nonce(&mut self, nonce: impl Into<MessageNonce>) {
        self.params.nonce = Some(nonce.into());
    }

    /// Enables text to speech for this message.
    pub fn tts(&mut self) {
        self.params.tts = true;
    }

    /// Sets the embed of the post.
    pub fn embed(&mut self, embed: impl Into<Embed<'a>>) {
        self.params.embed = Some(embed.into());
    }

    /// Attaches a file to the message.
    pub fn file(&mut self, file: CreateMessageFile<'a>) {
        self.files.push(file);
    }
}

fut_builder! {
    ('a, invite_fut_mod, ChannelOps, self)

    /// A future for creating a new invite to a channel.
    ///
    /// Instances can be obtained via [`ChannelOps::invite`].
    struct InviteFut {
        params: CreateChannelInviteParams<'a>,
        explicit_unique: Option<bool>,
    }
    into_async!(|ops, mut data| -> Result<Invite> {
        if let Some(explicit_unique) = data.explicit_unique {
            data.params.unique = Some(explicit_unique);
        } else {
            data.params.unique = Some(match data.params.max_uses {
                Some(x) => x != 0,
                None => false,
            });
        }
        ops.raw.create_channel_invite(ops.id, data.params).await
    });

    /// Sets the number of seconds this invite is valid for. If set to zero, the invite will never
    /// expire.
    pub fn expires_in(&mut self, age: u32) {
        self.params.max_age = Some(age);
    }

    /// Sets the invite to never expire.
    pub fn no_expire(&mut self) {
        self.params.max_age = Some(0);
    }

    /// Sets the maximum number of times this invite can be used. If set to zero, the invite can be
    /// used any number of times.
    pub fn max_uses(&mut self, uses: u32) {
        self.params.max_uses = Some(uses);
    }

    /// Sets whether the user should be invited temporary.
    ///
    /// Users invited with a temporary invite are kicked when they disconnect unless they have
    /// been assigned any roles.
    pub fn temporary(&mut self, temporary: bool) {
        self.params.temporary = Some(temporary);
    }

    /// Sets whether to reuse older invite codes.
    ///
    /// This is true by default, and is disabled by default if `max_uses` is set.
    pub fn reuse(&mut self, reuse: bool) {
        self.explicit_unique = Some(!reuse);
    }
}

fut_builder! {
    ('a, emoji_reactions_fut_mod, MessageOps, self)

    /// A future for creating retrieving a list of users who reacted a particular emoji
    /// to a message.
    ///
    /// Instances can be obtained via [`MessageOps::emoji_reactions`].
    params!(emoji: &'a EmojiRef);
    struct EmojiReactionsFut {
        params: GetReactionsParams<'a>,
    }
    into_async!(|ops, data| -> Result<Vec<User>> {
        ops.raw.get_reactions(ops.channel_id, ops.message_id, data.emoji, data.params).await
    });

    /// Gets reactions by users before the user ID.
    ///
    /// Mutually exclusive with `after`.
    pub fn before(&mut self, id: impl Into<UserId>) {
        self.params.before = Some(id.into());
    }

    /// Gets reactions by users after the user ID.
    ///
    /// Mutually exclusive with `before`.
    pub fn after(&mut self, id: impl Into<UserId>) {
        self.params.after = Some(id.into());
    }

    /// Sets the number of users to return.
    ///
    /// Currently limited to 1-100 users. Defaults to 25 users.
    pub fn limit(&mut self, limit: u32) {
        self.params.limit = Some(limit);
    }
}

fut_builder! {
    ('a, edit_fut_mod, MessageOps, self)

    /// A future for editing a message.
    ///
    /// Instances can be obtained via [`MessageOps::edit`].
    struct EditFut {
        params: EditMessageParams<'a>,
    }
    into_async!(|ops, data| -> Result<Message> {
        ops.raw.edit_message(ops.channel_id, ops.message_id, data.params).await
    });

    /// Sets the content of the post.
    pub fn content(&mut self, content: impl Into<Cow<'a, str>>) {
        self.params.content = Some(content.into());
    }

    /// Sets the embed of the post.
    pub fn embed(&mut self, embed: impl Into<Embed<'a>>) {
        self.params.embed = Some(embed.into());
    }

    /// Sets the new flags on this post.
    ///
    /// Note that this should be a complete copy of all flags the message should have, even those
    /// that cannot be changed by bots, for future compatibility reasons.
    pub fn flags(&mut self, flags: impl Into<EnumSet<MessageFlag>>) {
        self.params.flags = Some(flags.into());
    }
}
