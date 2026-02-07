//! Data models for ServiceDesk Plus API.
//!
//! This module contains type definitions for the SDP API, including
//! request/ticket models, technician models, note models, conversation models,
//! and common response types.

mod common;
mod conversation;
mod note;
mod request;
mod technician;

pub use common::*;
pub use conversation::*;
pub use note::*;
pub use request::*;
pub use technician::*;
