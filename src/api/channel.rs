use crate::errors::*;
use crate::http::*;
use crate::model::channel::*;
use crate::model::message::*;
use crate::model::types::*;
use std::borrow::Cow;

/// Performs operations relating to a Discord channel.
///
/// Instances can be obtained by calling [`DiscordContext::channel`]
pub struct ChannelOps<'a> {
    pub(crate) id: ChannelId,
    pub(crate) raw: Routes<'a>,
}
impl <'a> ChannelOps<'a> {
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
    /// async fn test_fn(ctx: DiscordContext, id: ChannelId) -> Result<()> {
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



    /// Retrieves a message from the channel.
    pub async fn get_message(self, id: MessageId) -> Result<Message> {
        self.raw.get_channel_message(self.id, id).await
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
    /// A value of 0 represents no rate limit.
    ///
    /// Currently limited to 0-21600 seconds.
    ///
    /// Only available for text channels.
    pub fn rate_limit(&mut self, rate_limit: u32) {
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
    pub fn category(&mut self, parent: Option<ChannelId>) {
        self.params.parent_id = Some(parent);
    }
}

fut_builder! {
    ('a, get_message_history_mod, ChannelOps, self)

    /// A future for a channel's message history.
    ///
    /// Instances can be obtained via [`ChannelOps::get_message_history`].
    struct GetMessagesHistoryFut {
        params: GetChannelMessagesParams<'a>,
    }
    into_async!(|ops, data| -> Result<Vec<Message>> {
        ops.raw.get_channel_messages(ops.id, data.params).await
    });

}