# Bittorrent downloader written by rust

This project is relevant bittorrent protocol.

Current support a [`tui`](https://github.com/kyrosle/bt-tui) program to use the downloader. I will support the win/linux/android platform
by using [`yew`](https://github.com/yewstack/yew) + [`tauri`](https://github.com/tauri-apps/tauri), that is, the application will wholly written by rust program language(with some js code for
wasm-bingen).

Create Entity Entity:
```mermaid
graph TB
engine_spawn--> EngineHandle
engine_spawn--> AlertReceiver
EngineHandle-->|new|Engine
Engine-->|joinhandle + engine_tx|EngineHandle
Engine-->|new|Disk
Disk-->|jonhandle + disk_tx|Engine
```


Create a Torrent file instance:
```mermaid
graph TB 
torrent_params-->|pass|Engine
Engine---engine_tx
engine_tx-->|create_torrent|Torrent
engine_tx-->|NewTorrent|Disk
Torrent-->|insert TorrentEntity|Engine
```

Components channels:
```mermaid
graph TB
Conf-->Engine
Engine==>|create|Alert_rx
Engine-->|create|Disk
Engine-->|create|TorrentA
Engine-->|create|TorrentB
Engine-->|Command Send|Disk
TorrentA-->|Command Send|Disk
TorrentB-->|Command Send|Disk
Engine-->|Command Send|TorrentA
Engine-->|Command Send|TorrentB
Disk==>|TorrentComplete or TorrentStats or Error|Alert_rx
Engine==>|TorrentComplete or TorrentStats or Error|Alert_rx
TorrentA==>|TorrentComplete or TorrentStats or Error|Alert_rx
TorrentB==>|TorrentComplete or TorrentStats or Error|Alert_rx
```


Shutdown() the engine and wait their joinhandle:
```mermaid
graph TB
EngineHandle-->|engine_tx|Engine
Engine-->|torrent_tx|TorrentA
Engine-->|torrent_tx|TorrentB
Engine-->|disk_tx|Disk
```