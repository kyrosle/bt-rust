use bt_rust::metainfo::Metainfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = r#"fixtures\debian-iso.torrent"#;
    let v = std::fs::read(path)?;
    let metainfo = Metainfo::from_bytes(&v).unwrap();
    //println!("{metainfo:?}");
    Ok(())
}
