use std::net::IpAddr;

use crate::{PeerId, Sha1Hash};

/// Parameters for announcing to a tracker (for request params).
/// [`More details about the key meanings`](http://bittorrent.org/beps/bep_0003.html)
pub struct Announce {
    /// info_hash from torrent file.
    pub info_hash: Sha1Hash,
    /// self identifier
    pub peer_id: PeerId,

    /// the port program is listening.
    pub port: u16,
    /// the true ip address in dotted quad format.
    /// Only necessary when : HTTP request originated is not the same as
    /// the client's host address.
    /// (client communication through a proxy or when the tracker is on the
    /// same NAT'd subset as peer)
    pub ip: Option<IpAddr>,

    /// Number up bytes download so far.
    pub downloaded: u64,
    /// Number of bytes uploaded so far.
    pub uploaded: u64,
    /// Number of bytes left to download.
    pub left: u64,

    /// The number of peers the client wishes to receive from the tracker. If omitted and
    /// the tracker is UDP, -1 is sent to signal the tracker to determine the number of
    /// peers, and if it's omitted and the tracker is HTTP, this is typically swapped
    /// for a value between 30 and 50.
    pub peer_count: Option<usize>,

    /// If previously received from the tracker, we must send it with each
    /// announce.
    #[allow(dead_code)]
    pub tracker_id: Option<String>,

    /// Only need be set during the special events defined in [`Event`].
    /// Otherwise when just requesting peers, no event needs to be set.
    #[allow(dead_code)]
    pub event: Option<Event>,
}

/// The optional announce event.
///
/// If not present, the event will be the `Empty` type.
///
/// If not present, this is one of the announcements done at regular intervals.
pub enum Event {
    /// The first request to tracker must include this value.
    Started,
    /// Must be sent to the tracker when the client becomes a seeder.
    /// Must not be present if the client started as a seeder(who finish the download).
    Completed,
    /// Must be sent to tracker if the client is shutting down gracefully.
    Stopped,
}
