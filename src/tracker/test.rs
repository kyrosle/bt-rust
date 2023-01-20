/// test the tracker module correctly.
#[cfg(test)]
mod tests {
    use std::{net::{Ipv4Addr, SocketAddr}, time::Duration};

    use mockito::{Matcher, mock};
    use serde_derive::{Deserialize, Serialize};

    use crate::tracker::prelude::*;

    #[derive(Deserialize)]
    struct PeersResponse {
        #[serde(deserialize_with = "deserialize_peers")]
        peers: Vec<SocketAddr>,
    }

    #[test]
    fn should_parse_compact_peer_list() {
        let ip = Ipv4Addr::new(192, 168, 0, 1);
        let port = 8989;

        // build up encoded byte string
        let mut encoded = Vec::new();
        encoded.extend_from_slice(b"d5:peers");
        encoded.extend_from_slice(&encode_compact_peers_list(&[(ip, port)]));
        encoded.push(b'e');

        let decoded: PeersResponse =
            serde_bencode::from_bytes(&encoded).expect("cannot decode bencode string of peers");
    
        let addr = SocketAddr::new(ip.into(), port);

        assert_eq!(decoded.peers, vec![addr]);
    }
    
    #[test]
    fn should_parse_full_peer_list() {
        #[derive(Debug, Serialize)]
        struct RawPeer {
            ip: String,
            port: u16,
        }

        #[derive(Debug, Serialize)]
        struct RawPeers {
            peers: Vec<RawPeer>,
        }

        let peers = RawPeers {
            peers: vec![
                RawPeer {
                    ip: "192.168.1.10".into(),
                    port: 55123,
                },
                RawPeer {
                    ip: "1.45.96.2".into(),
                    port: 1234,
                },
                RawPeer {
                    ip: "123.123.123.123".into(),
                    port: 49950,
                },
            ],
        };

        let encoded = serde_bencode::to_string(&peers).unwrap();

        let decoded: PeersResponse = serde_bencode::from_str(&encoded)
            .expect("cannot decode bencode list of peers");
        let expected: Vec<_> = peers
            .peers
            .iter()
            .map(|p| SocketAddr::new(p.ip.parse().unwrap(), p.port))
            .collect();
        assert_eq!(decoded.peers, expected);
    }

    #[tokio::test]
    async fn should_return_peers_on_announce() {
        let addr = mockito::server_url();
        let tracker = Tracker::new(addr.parse().unwrap());

        let info_hash_str = "abcdefghij1234567890";
        let mut info_hash = [0; 20];
        info_hash.copy_from_slice(info_hash_str.as_bytes());

        let peer_id_str = "cbt-2020-03-03-00000";
        let mut peer_id = [0; 20];
        peer_id.copy_from_slice(peer_id_str.as_bytes());

        // ready announce to send.
        let announce = Announce {
            info_hash,
            peer_id,
            port: 16,
            downloaded: 1234,
            uploaded: 1234,
            left: 1234,
            peer_count: Some(2),
            ip: None,
            event: None,
            tracker_id: None,
        };

        // tracker provide useable peer.
        let peer_ip = Ipv4Addr::new(2, 156, 201, 254);
        let peer_port = 49123;

        // client expected receive from tracker server.
        let expected_resp = Response {
            tracker_id: None,
            failure_reason: None,
            warning_message: None,
            interval: Some(Duration::from_secs(15)),
            min_interval: Some(Duration::from_secs(10)),
            seeder_count: Some(5),
            leecher_count: Some(3),
            peers: vec![SocketAddr::new(peer_ip.into(), peer_port)],
        };

        // expected_response -> bencode
        let mut encoded_resp = Vec::new();
        // unterminated dict
        encoded_resp.extend_from_slice(
            b"d\
            8:completei5e\
            10:incompletei3e\
            8:intervali15e\
            12:min intervali10e",
        );
        // insert peers field into dict
        encoded_resp.extend_from_slice(b"5:peers");
        encoded_resp.extend_from_slice(&encode_compact_peers_list(&[(
            peer_ip, peer_port,
        )]));
        // terminate dict
        encoded_resp.push(b'e');

        // register the mock server. 
        // (receive the specified announce and return the specified expected-response) 
        // both in bencode.
        let _m = mock("GET", "/")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("compact".into(), "1".into()),
                Matcher::UrlEncoded("info_hash".into(), info_hash_str.into()),
                Matcher::UrlEncoded("peer_id".into(), peer_id_str.into()),
                Matcher::UrlEncoded("port".into(), announce.port.to_string()),
                Matcher::UrlEncoded(
                    "downloaded".into(),
                    announce.downloaded.to_string(),
                ),
                Matcher::UrlEncoded(
                    "uploaded".into(),
                    announce.uploaded.to_string(),
                ),
                Matcher::UrlEncoded("left".into(), announce.left.to_string()),
                Matcher::UrlEncoded(
                    "numwant".into(),
                    announce.peer_count.unwrap().to_string(),
                ),
            ]))
            .with_status(200)
            .with_body(encoded_resp)
            .create();
        

        let resp = tracker.announce(announce).await.unwrap();
        assert_eq!(resp, expected_resp);
    }

    fn encode_compact_peers_list(peers: &[(Ipv4Addr, u16)]) -> Vec<u8> {
        let encoded_peers: Vec<_> = peers
            .iter()
            .map(|(ip, port)| {
                ip.octets()
                    .iter()
                    .chain([(port >> 8) as u8, (port & 0xff) as u8].iter())
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect();

        let mut encoded = Vec::new();
        encoded.extend_from_slice(encoded_peers.len().to_string().as_bytes());
        encoded.push(b':');
        encoded.extend_from_slice(&encoded_peers);

        encoded
    }
}
