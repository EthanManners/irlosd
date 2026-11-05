# irlosd

One IRLOS seat: irlosd runs IrlosStudio headless on the GPU (NVENC, no X, no VNC) and
controls it over ICP. It replaces NOALBS and the old Xorg + dummy plug + XFCE + x11vnc stack.

```
field encoder ══SRT══▶ ingest (SLS) ══▶ IrlosStudio --headless ══RTMPS══▶ Kick / Twitch
                         │ stats                ▲ ICP (obs-control.sock)
                         └────────▶ irlosd ─────┘
                                      ▲ ICP (irlosd.sock)
                               console · web panel (later)
```

## Go live

```sh
cd ~/irlosd
nano .env          # STREAM_KEY=<your Kick stream key>
./irlosd           # launches irlosd and opens the console; press s
```

With no feed configured, the stream shows the BRB test card. `q` closes the console;
irlosd and the stream keep running. `./irlosd start | stop | status | scene <name>` do the
same without the console.

## What irlosd does

- **Supervises IrlosStudio.** Rebuilds its config from `studio/` before every launch, relaunches
  it with backoff if it exits, and stops it gracefully on `docker compose stop`.
- **Keeps the stream where you asked.** After a start, if the stream ends up down (Studio crashed,
  reconnects ran out), irlosd starts it again. Only a stop request (or stopping irlosd) ends a stream.
- **Switches scenes from the feed.** If `SLS_STATS_URL` is set, it polls SRT Live Server once a
  second: Live, Low (bitrate ≤ `LOW_KBPS` or RTT ≥ `LOW_RTT_MS`) or BRB (no publisher). It takes 3
  bad samples to degrade and 5 good ones to recover. Switches happen only when feed health
  changes, so a manual scene choice holds until then.
- **Speaks ICP to clients.** `/run/irlos/irlosd.sock` uses the same framing, handshake and ops as
  IrlosStudio's plugin (`IrlosStudio/plugins/irlos-control/protocol.md`), plus `get_status`.
  Other ops pass through to IrlosStudio. Any number of clients can connect; irlosd queues their
  requests over its single connection to IrlosStudio. The web control panel should talk to irlosd
  the same way.

## Settings (`.env`)

| Key | Default | |
|---|---|---|
| `STREAM_KEY` | — | Required to go live |
| `STREAM_SERVER` | Kick ingest | Any RTMP(S) URL |
| `SRT_URL` | empty | Feed for the Live/Low scenes, e.g. `srt://relay:4000?streamid=play/live/feed1` |
| `SLS_STATS_URL`, `SLS_PUBLISHER` | off, `publish/live/feed1` | Auto switching |
| `LOW_KBPS`, `LOW_RTT_MS`, `OFFLINE_KBPS` | 800, 2500, 0 (off) | Switching thresholds |
| `RESOLUTION`, `FPS`, `BITRATE_KBPS` | 1920x1080, 30, 6000 | NVENC H.264 CBR, 2 s keyframes |

`.env` changes take effect on the next `./irlosd` (compose recreates the container).

## Layout

- `src/` is the daemon: `icp.rs` (protocol), `daemon.rs` (supervisor, poller, ICP server),
  `studio.rs` (config + process), `feed.rs` (SLS + debounce), `console.rs`, `config.rs`.
- `studio/obs-studio/` holds the IrlosStudio profile and scene templates (`{{NAME}}` is filled
  from `.env`). The scenes are `Live`, `Low` and `BRB`.
- `Dockerfile` builds IrlosStudio from `IrlosStudio/` and irlosd into one Ubuntu 24.04 image.
  `compose.yaml` runs it with the NVIDIA GPU.

## GPU server setup (done once)

Ubuntu 22.04, NVIDIA 595 driver, Docker CE, NVIDIA Container Toolkit
(`nvidia-ctk runtime configure --runtime=docker`). The user is in the `docker` group.
Build with `docker compose build` (about 5 minutes).
