//! Defines a convenient to use API for making calls to the Discord API.
//!
//! Most of the types defined here are not useful on their own, and are obtained by calling
//! methods on [`DiscordContext`].

use crate::context::*;
use crate::model::channel::{Channel, PartialChannel};
use crate::model::guild::{Guild, PartialGuild};
use crate::model::message::Message;
use crate::model::types::*;

// TODO: Create iterators based on the various get_* functions.
// TODO: Consider various API extensions based on full message/user/etc objects.
// TODO: Solidify the ordering of the reason/token methods in call chains, etc.

macro_rules! fut_builder {
    (
        ($lt:lifetime, $mod_name:ident, $parent_name:ident, $self_ident:ident)
        $(#[$struct_meta:meta])*
        $(params!($($struct_param_name:ident: $struct_param_ty:ty),* $(,)?);)?
        struct $ops_name:ident {
            $($field_name:ident: $field_ty:ty),* $(,)?
        }
        into_async!(|$parent:pat, $data:pat| -> $async_ty:ty {
            $($async_body:tt)*
        });
        $(
            $(#[$fn_meta:meta])*
            $fn_vis:vis fn $fn_name:ident(
                &mut self $(, $param_name:ident: $param_ty:ty)* $(,)?
            ) {
                $($fn_body:tt)*
            }
        )*
    ) => {
        mod $mod_name {
            use super::*;
            use std::future::Future;
            use std::pin::Pin;
            use std::task::{Poll, Context};

            async fn fut_fn<$lt>(
                $parent: $parent_name<$lt>, $data: Data<$lt>,
            ) -> $async_ty {
                $($async_body)*
            }
            fn make_fut<$lt>(parent: $parent_name<$lt>, data: Data<$lt>) -> FutType<$lt> {
                let fut = fut_fn(parent, data);
                #[cfg(not(feature = "nightly"))]
                let fut = Box::new(fut);
                fut
            }

            #[cfg(feature = "nightly")]
            type FutType<$lt> = impl Future<Output = $async_ty> + Send + $lt;

            #[cfg(not(feature = "nightly"))]
            type FutType<$lt> = Box<dyn Future<Output = $async_ty> + Send + $lt>;

            struct Data<$lt> {
                $($($struct_param_name: $struct_param_ty,)*)?
                $($field_name: $field_ty,)*
            }
            enum State<$lt> {
                Builder($parent_name<$lt>, Data<$lt>),
                Future(FutType<$lt>),
                TempInvalid,
            }

            $(#[$struct_meta])*
            #[must_use]
            #[doc = "\n\nThis struct doubles as a future and a builder. It serves as a builder \
                         until it is awaited or polled, at which point all further attempts to \
                         call builder methods will panic."]
            pub struct $ops_name<$lt>(State<$lt>);

            impl <$lt> Data<$lt> {
                $(
                    #[allow(unused_mut)]
                    fn $fn_name(&mut $self_ident, $($param_name: $param_ty,)*) {
                        $($fn_body)*
                    }
                )*
            }
            impl <$lt> $ops_name<$lt> {
                pub(crate) fn new(
                    parent: $parent_name<$lt>, $($($struct_param_name: $struct_param_ty,)*)?
                ) -> Self {
                    $ops_name(State::Builder(parent, Data {
                        $($($struct_param_name,)*)?
                        $($field_name: Default::default(),)?
                    }))
                }
                fn retrieve_parent(&mut self) -> &mut $parent_name<$lt> {
                    match &mut self.0 {
                        State::Builder(parent, _) => parent,
                        State::Future(_) =>
                            panic!("This method may not be called after this future is polled."),
                        State::TempInvalid => unreachable!(),
                    }
                }
                fn retrieve_builder(&mut self) -> &mut Data<$lt> {
                    match &mut self.0 {
                        State::Builder(_, data) => data,
                        State::Future(_) =>
                            panic!("This method may not be called after this future is polled."),
                        State::TempInvalid => unreachable!(),
                    }
                }
                fn into_fut(
                    self: Pin<&mut Self>
                ) -> Pin<&mut (impl Future<Output = $async_ty> + ?Sized + $lt)> {
                    unsafe {
                        let self_mut = &mut self.get_unchecked_mut().0;
                        if let State::Builder(_, _) = self_mut {
                            match ::std::mem::replace(self_mut, State::TempInvalid) {
                                State::Builder(parent, data) =>
                                    *self_mut = State::Future(make_fut(parent, data)),
                                _ => unreachable!(),
                            }
                        }
                        match self_mut {
                            State::Future(fut) => {
                                #[cfg(not(feature = "nightly"))]
                                let fut = &mut **fut;
                                Pin::new_unchecked(fut)
                            },
                            _ => unreachable!(),
                        }
                    }
                }

                $(
                    $(#[$fn_meta])*
                    #[allow(unused_mut)]
                    #[allow(dead_code)]
                    $fn_vis fn $fn_name(mut self, $($param_name: $param_ty,)*) -> Self {
                        self.retrieve_builder().$fn_name($($param_name,)*);
                        self
                    }
                )*

                routes_wrapper!(self, &mut self.retrieve_parent().raw);
            }
            impl <$lt> Future for $ops_name<$lt> {
                type Output = $async_ty;
                fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                    Future::poll(self.into_fut(), cx)
                }
            }
        }
        pub use $mod_name::$ops_name;
    };
}

mod channel;
mod guild;
mod user;

pub use channel::*;
pub use guild::*;
pub use user::*;

impl DiscordContext {
    /// Performs operations relating to a Discord channel.
    pub fn channel(&self, id: impl Into<ChannelId>) -> ChannelOps<'_> {
        ChannelOps { id: id.into(), raw: self.raw() }
    }

    /// Performs operations relating to a message.
    pub fn message(
        &self, channel: impl Into<ChannelId>, message: impl Into<MessageId>,
    ) -> MessageOps<'_> {
        MessageOps { channel_id: channel.into(), message_id: message.into(), raw: self.raw() }
    }

    /// Performs operations relating to a guild.
    pub fn guild(&self, id: impl Into<GuildId>) -> GuildOps<'_> {
        GuildOps { id: id.into(), raw: self.raw() }
    }

    /// Performs operations relating to a member.
    pub fn member(&self, guild: impl Into<GuildId>, member: impl Into<UserId>) -> MemberOps<'_> {
        MemberOps { guild_id: guild.into(), user_id: member.into(), raw: self.raw() }
    }
}
impl Channel {
    /// Performs operations on this channel.
    pub fn ops<'a>(&self, ctx: &'a DiscordContext) -> ChannelOps<'a> {
        ChannelOps { id: self.id, raw: ctx.raw() }
    }
}
impl Guild {
    /// Performs operations on this guild.
    pub fn ops<'a>(&self, ctx: &'a DiscordContext) -> GuildOps<'a> {
        GuildOps { id: self.id, raw: ctx.raw() }
    }
}
impl PartialChannel {
    /// Performs operations on this channel.
    pub fn ops<'a>(&self, ctx: &'a DiscordContext) -> ChannelOps<'a> {
        ChannelOps { id: self.id, raw: ctx.raw() }
    }
}
impl PartialGuild {
    /// Performs operations on this guild.
    pub fn ops<'a>(&self, ctx: &'a DiscordContext) -> GuildOps<'a> {
        GuildOps { id: self.id, raw: ctx.raw() }
    }
}
impl Message {
    /// Performs operations on this message.
    pub fn ops<'a>(&self, ctx: &'a DiscordContext) -> MessageOps<'a> {
        MessageOps { channel_id: self.channel_id, message_id: self.id, raw: ctx.raw() }
    }
}