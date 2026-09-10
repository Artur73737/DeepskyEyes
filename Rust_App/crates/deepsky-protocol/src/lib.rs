//! DeepskyEyes wire protocol — solo tipi, versione, framing (README §25).
//! Non conosce GPUI ne' Camera2.

pub mod version;
pub mod header;
pub mod framing;
pub mod codec;
pub mod commands;
pub mod responses;
pub mod events;
pub mod errors;
pub mod wire_types;
pub mod rpc;
