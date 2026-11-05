# irlosd — Consolidated 13-Step Implementation Roadmap

**Status:** draft for owner review, revision 2. **Drafted:** 2026-11-02. **Revised:** 2026-11-03 (audit and revision; see §7). **Nothing in this file is implemented.**
**Baseline:** `irlosd` at commit `dab2665` (an unmodified NOALBS 2.21.1 clone; the package is still named `noalbs`) and IrlosStudio branch `irlos/icp-plugin` at `564647bcb`. Neither repository changed between revision 1 and revision 2.
**Replaces:** the 16-step plan and its eleven step prompts (§7). Those files were left unchanged.

**Contents**

- Part I — Roadmap overview: §1 Purpose and end state · §2 How to use this roadmap · §3 Step table · §4 Dependencies, critical path and ordering decision · §5 Architecture · §6 Environment summary · §7 Supersession and revision note
- Part II — Shared standards: §8 Contract registry · §9 Development and live-testing workflow
- Part III — Step prompts 01–13. Each step is wrapped in `<!-- STEP NN START -->` / `<!-- STEP NN END -->` and can be handed to an agent together with Parts I and II.
- Part IV — Appendices: A old → new mapping · B open questions (`Q-NN`) · C decisions needing owner review (`D-NN`) · D source documents · E glossary

---

## Part I — Roadmap overview

### 1. Purpose and end state

IRLOS sells "one button to stream".
- The streamer's field encoder (a phone, camera or Belabox) sends one SRT feed to an ingest point. That is either the IRLOS relay running SRT Live Server (sls) or, for some tenants, Belabox cloud.
- An IRLOS GPU server pulls the feed into IrlosStudio (the IRLOS fork of OBS Studio 30.2.3), composes the broadcast and pushes it over RTMP to Twitch or Kick.
- The fragile hop is field → ingest. When it degrades, the server cuts to a LOW or BRB scene while the platform output keeps running, so viewers see a holding card instead of a dropped stream.

Today a human operator does this work: they provision each customer by hand, watch streams over VNC, switch scenes and restart crashed software. The legacy tool NOALBS (`noalbs`) automates only scene switching, and it does so from outside OBS.

**`irlosd` replaces the operator.** When this roadmap is complete, one `irlosd` process runs per customer session ("seat") and does the following:

1. **Supervises IrlosStudio.** It generates a private profile and scene collection, launches Studio on a private display, restarts it within a bounded budget and stops it cleanly.
2. **Controls IrlosStudio through one controller interface.** The primary driver is ICP (IRLOS Control Protocol) over a private Unix socket, served by Studio's `irlos-control` plugin. OBS WebSocket v5 is a fallback chosen explicitly in configuration, never by automatic failover.
3. **Ingests telemetry.** It polls SRT Live Server statistics (the primary source for relay tenants) and Belabox statistics (for tenants fed through Belabox cloud). It normalizes them and never invents values.
4. **Owns the session's truth.** One state-owner actor holds desired state (what humans and policy asked for) and observed state (what Studio, the process and telemetry report). It reconciles them through one serialized Studio executor.
5. **Switches scenes automatically.** A pure hysteresis policy inside the actor needs 3 bad samples to degrade and 5 good samples to recover. Stale data falls back to a safe BRB scene, and stalled media is repaired under guards. Outputs are never stopped because ingest is poor.
6. **Accepts commands from three kinds of client.** Authenticated streamer commands arrive from Twitch and Kick chat. Local operator commands and status queries arrive over an admin Unix socket. An attach-only terminal interface (`irlosd tui`) uses that socket. All three submit intents to the state owner; none of them drives Studio.
7. **Runs isolated.** Each session runs in its own container with GPU encoding and its own display, sockets, files and credentials. Several seats share one GPU server. The owner's target is 8 seats per server, subject to measured capacity.

irlosd never carries video. Media flows field → ingest → IrlosStudio → platforms; irlosd carries only control and measurements.

Out of scope here:
- the customer dashboard;
- the IrlosWeb provisioning service (Stripe, web app, GPU-host agent).

Both are future clients. Step 12 gives the IrlosWeb GPU-host agent a seat tool whose vocabulary matches the agent's frozen contract (`provision`, `suspend`, `destroy`, `status`, `capacity`, `control`), so the agent can call it.

**Where things stand.**
- irlosd is upstream NOALBS with no IRLOS changes.
- The IrlosStudio `irlos-control` plugin implements the ICP v1.0 base operations, a test server (`icp_testserver`) and a Python conformance suite. No test results for any of them are recorded.
- irlosd and the plugin have never been run together.
- No container, supervisor, admin socket or TUI code exists anywhere.

### 2. How to use this roadmap

**For the owner.** Before Step 01 starts:
- Review Appendix C (decisions needing owner review) and Appendix B (open questions).
- Decide the owner-approved actions in §9.9. Most important are using `ssh GPU` for development at all (it is the production customer host, §6), the IrlosStudio build route (D-09) and container support.

Every step prompt tells the implementing agent to stop and ask before any of these.

**For an implementing agent.** You receive Part I, Part II and one step prompt.

1. Read Parts I and II first. They define every shared name, path, type, configuration key, event name and the workflow. Do not invent alternatives.
2. Implement steps in numerical order. Before starting, confirm the step's *Starting state* by reading the code and running `live/live.sh check all`. If an earlier step's checks fail, fix or report that first.
3. Work in units. After each unit:
   - run the local tests;
   - sync to `ssh GPU` and build there;
   - run the step's live checks plus all earlier checks;
   - record the run (§9.4–§9.6).
4. **"Done" means you ran it and can show the log line.** This is the owner's rule from the onboarding guide. A step is done only when:
   - its acceptance criteria hold;
   - its live checks pass on `ssh GPU`;
   - every earlier live check still passes;
   - the evidence is in `live/LIVE_RUNS.md`.

   "It compiled" or "the tests should pass" is not evidence.
5. Do not change a contract in §8. If one turns out to be wrong, stop and ask the owner: other steps depend on it.
6. The code beats this document on facts. If the code differs from a *Starting state*, record the difference in the run log and in your report, and keep the step's intent.
7. Stop and ask before any owner-approved action (§9.9), any broadcast to a real platform, or any change outside the `irlosd` repository. IrlosStudio C++ changes count as outside.
8. `ssh GPU` hosts paying customers' streams. Never touch a process, display, port, file or GPU context you did not create. Run the safety preflight (§9.8) before every GPU-using check.
9. Never copy secret values into code, fixtures, logs, commits or reports. Use fake values such as `<TWITCH_BOT_TOKEN>`.

### 3. Step table

| # | Title | One-line objective | Depends on | Absorbs old steps | Code status at drafting (evidence) |
|---|---|---|---|---|---|
| 01 | Foundation and live-test workflow | Turn the NOALBS clone into the `irlosd` package with IDs, one error model, one configuration model, logging, CLI dispatch and the local + `ssh GPU` workflow | — | 1, 2, new workflow | **Not started.** Package `noalbs` 2.21.1 (`Cargo.toml:2-3`); 17-variant legacy `Error` (`src/error.rs`); no CLI parsing (`src/main.rs`); startup GitHub release check (`src/main.rs:151`); no `live/` or `tests/` |
| 02 | ICP v1 protocol layer | Rust framing, typed messages and handshake matching `protocol.md` v1.0 and the plugin | 01 | 3, 4 | **Not started** in irlosd (no `src/broadcasting_software/icp/`). Server side implemented in IrlosStudio (`plugins/irlos-control/src/icp/*`), never run |
| 03 | Broadcasting-controller interface | One controller trait; ICP Unix-socket client (primary), OBS v5 bridge (fallback), factory | 01–02 | 5, 6 | **Partial.** Legacy trait (`src/broadcasting_software/mod.rs:8-54`) and `Obsv5` (`obs_v5.rs`, 952 lines) use fuzzy scene matching, hold a lock across a 2 s sleep and can hit an `unimplemented!` panic. No ICP client. `src/noalbs.rs:59-64` always builds `Obsv5` |
| 04 | IrlosStudio supervision and profile generation | Single owner of the Studio process; private generated profiles; bounded restart; clean stop | 01–03 | 10 | **Not started.** No process code; Tokio lacks `process` (`Cargo.toml:27`) |
| 05 | Authoritative state owner and `serve` | One actor with desired and observed state behind `SessionHandle`; reconciler; startup and shutdown; legacy runtime removed | 01–04 | 11 (core) | **Not started as specified.** Legacy `Arc<RwLock<State>>` has several writers (`src/state.rs`, `src/noalbs.rs:15`) |
| 06 | Local admin IPC and `status`/`command` | Framed JSON-RPC admin socket and CLI clients over `SessionHandle` | 05 | 13 | **Not started.** Legacy Warp `/ws` API (`src/web_server.rs`, `src/ws/`) is removed in 01/05 |
| 07 | Telemetry ingestion | SLS and Belabox providers with truthful normalized outcomes | 05–06 | 7, 8 | **Partial.** Legacy `sls.rs` (178 lines) and `belabox.rs` (144 lines) re-fetch on every call; unreachable maps to "Offline" |
| 08 | Scene policy and media repair | Pure 3/5 hysteresis, Auto vs holds, stale-data BRB, guarded media repair | 05–07 | 9, 11 (policy parts) | **Partial.** Legacy `switcher.rs` (507 lines) uses one symmetric `retry_attempts` and drives Studio directly |
| 09 | Chat command gateway | Authenticated Twitch and Kick commands become intents, with permissions, rate limits and replies | 05–08 | 12 | **Partial (legacy).** `src/chat/`: Twitch IRC works but lowercases the OAuth token (`twitch.rs:32-33`); username-based admins (`chat_handler.rs:299`); Kick send only logs (`kick.rs:72-74`) |
| 10 | Interactive terminal interface | Attach-only `irlosd tui` over the admin socket | 06 (data from 07–09) | 14 | **Not started.** No ratatui or crossterm |
| 11 | Single-session container | Seat image with IrlosStudio, irlosd, a private display and NVENC | 01–10 | 15 (part) | **Not started.** No Dockerfile, compose file or unit in either repo |
| 12 | Multi-tenant isolation and seat lifecycle | Several isolated seats per host; `seatctl` with the IrlosWeb agent's vocabulary | 11 | 15 (rest) | **Not started** |
| 13 | End-to-end verification and alpha release | Scenario matrix, fault injection, soak, manual platform checklist, release criteria | 01–12 | 16 | **Not started** |

### 4. Dependencies, critical path and the ordering decision

```
 01 Foundation + live workflow ─────────────────────────────────────────┐
  │                                                                     │ every step
 02 ICP v1 protocol layer                                               │ extends
  │                                                                     │ live/live.sh
 03 Controller interface (UnixSocketClient | Obsv5 behind one trait)    │
  │                                                                     │
 04 Studio supervision + profile generation                             │
  │
 05 State owner + `serve`  ── defines SessionHandle, EngineInput, SessionSnapshot
  │
 06 Admin IPC + status/command  ── the observation/drive tool for later live checks
  │
 07 Telemetry (SLS, Belabox)    ── facts into the actor
  │
 08 Scene policy + media repair ── automation inside the actor
  │
 09 Chat gateway                ── authenticated intents
  │
 10 TUI                         ── admin client
  │
 11 Single-session container    ── packages 01–10
  │
 12 Multi-tenant isolation + seat lifecycle (seatctl)
  │
 13 End-to-end verification + alpha release
```

**Critical path.** The order is linear: 01 → 02 → … → 13. Two overlaps are possible if the owner runs agents in parallel:
- Step 07's local work can start once Step 05 is done, but its live checks need Step 06.
- Step 10's layout work can start after Step 06, and its screens fill in as Steps 07–09 land.

The largest environment risk is retired early. Steps 01–04 must first get IrlosStudio building on `ssh GPU`, launch it on a private display and talk ICP to it.

**Ordering decision (drafting task §5).** The suggested grouping builds the switcher (5) and supervisor (6) before the state owner (7). The old plan did the same: it kept the legacy runtime disabled and tested pieces "through fake sinks" until old Step 11 (`architecture_verification_steps_01_11.md` §4.1). That conflicts with continuous live testing, because nothing can run end to end on `ssh GPU` until the state owner exists. It also invites a second executor: the legacy `Switcher::run()` drives Studio directly. This roadmap reorders the steps without changing the architecture:

1. **The state owner moves up to Step 05.** It comes directly after the protocol, controller and supervisor layers it composes, and before telemetry, policy and chat, which are its fact producers and clients. Those later steps then plug into the real composition root and are live-tested through the real daemon, with no throwaway runtimes. Old Step 11 is split:
   - Step 05 takes human intent, holds, outputs, lifecycle and shutdown.
   - Step 08 takes the automation: hysteresis, stale-data BRB and media repair.
2. **Supervision stays before the state owner (Step 04).** It is a narrow fact producer with its own interface (`SupervisorHandle` plus `watch<ProcessObservation>`), as old Step 10 was. Its live checks drive the real IrlosStudio through an ignored integration test; Step 05 then composes it.
3. **Admin IPC moves up to Step 06.** It depends only on `SessionHandle`. From Step 07 on, every live check observes and drives the daemon through `irlosd status --json` and `irlosd command …` instead of parsing logs.

This is listed for owner review as **D-01**. Revision 2 keeps it: the audit found no reason to change it.

### 5. Architecture

```
LEGEND   ══▶ media      ──▶ telemetry      ┄┄▶ control      ║ seat (container) boundary

 FIELD ENCODER ══SRT══▶ INGEST: relay (sls) or Belabox cloud
                          ║ SRT pull (stream id = the only secret)   │ HTTP stats (sls) / Belabox stats
 ║════════════════════════╬══════════════════ seat <sessionId> ══════╪═════════════════════════════════║
 ║                        ▼                                          ▼                                 ║
 ║  IrlosStudio (child of irlosd)               irlosd serve                                         ║
 ║   ffmpeg_source ══▶ scenes Live/Low/BRB       TelemetryRunner ──▶ EngineInput ──▶ STATE OWNER      ║
 ║        ══▶ NVENC encoder ══▶ RTMP output                                   (actor + policy)        ║
 ║                                                                               ┆ actions           ║
 ║   irlos-control plugin  ◀┄┄┄┄┄ ICP ┄┄┄┄┄┄┄┄┄┄┄┄ Studio executor ◀┄┄┄┄┄┄┄┄┄┄┄┄┄┄┘                   ║
 ║    <runtimeDir>/obs-control.sock          (BroadcastingSoftwareLogic: UnixSocketClient | Obsv5)  ║
 ║   obs-websocket (fallback only) ◀┄ OBS v5 ┄┘                                                      ║
 ║                                             Supervisor ┄┄ spawn / SIGINT / reap ┄┄▶ IrlosStudio   ║
 ║   private display (Xvfb dev | nvidia_xorg)  Admin server <runtimeDir>/irlosd.sock ┄┄▶ STATE OWNER ║
 ║                                             Chat gateway (Twitch IRC out, Kick webhook in) ┄┄▶ actor
 ║════════╪════════════════════════════════════════════▲══════════════════════════▲══════════════════║
          ║ RTMP(S)                                    ┆ status/command/tui       ┆ Kick webhooks via a
          ▼                                            ┆ (docker exec, local)     ┆ trusted host edge proxy
   TWITCH / KICK ingest ══▶ viewers         Twitch IRC (outbound TLS from the seat) · webhook socket only
```

- **Media path.** It lives inside IrlosStudio only. irlosd writes media URLs into generated private profiles (Step 04) and may ask for a media repair (Step 08). It never touches video or audio.
- **Telemetry path.** Pollers publish `PollOutcome` facts into the actor. They never call the controller.
- **Control path.** Every intent flows chat/admin/TUI/policy → actor → single Studio executor → controller → Studio. The supervisor is the only process owner.
- **Container boundary (Steps 11–12).** Only these cross it:
  - the selected GPU device and the driver libraries;
  - the seat volume;
  - outbound ingest, platform, chat, DNS and time traffic;
  - the webhook socket directory, reachable by the trusted edge proxy only;
  - `docker exec` for local operator attach.

  No Studio, obs-websocket or admin port is published. The host's `:0` display and the Docker socket never enter a seat.

### 6. Environment summary

**Local development machine** (observed 2026-11-02, read-only):

| Item | Observed |
|---|---|
| System | Ubuntu 24.04.4 LTS, kernel 7.0.0-38-generic, glibc 2.39, 24 CPUs, 15 GiB RAM |
| Present | `gcc`/`cc`, `pkg-config`, OpenSSL headers, `python3`, `rsync`, `git`, `ssh`, `jq`, `pdftotext` |
| Absent | `cargo`, `rustc`, `rustup`, `docker`, `podman`, `cmake`, `ffmpeg`, `Xvfb`, `musl-gcc` |
| IrlosStudio checkout | The `plugins/obs-websocket` and `plugins/obs-browser` submodules are empty; `plugins/CMakeLists.txt:12-31` fails configuration without them. No Git tags exist, so the version must come from `OBS_VERSION_OVERRIDE` (inferred from `cmake/common/versionconfig.cmake`) |
| IrlosStudio build needs | FFmpeg ≥ 6.1 (`libobs/CMakeLists.txt:12`), FFnvcodec ≥ 12 (`plugins/obs-nvenc/CMakeLists.txt:21`), CMake 3.28–3.30 (also for the standalone plugin, `plugins/irlos-control/CMakeLists.txt:6`), Qt 6, nlohmann_json 3. CI targets Ubuntu 24.04 (`.github/scripts/utils.zsh/setup_ubuntu`) |

**`ssh GPU` (live environment): not observed.** Five read-only connection attempts failed with `Connection timed out`: three during drafting on 2026-11-02 and two during revision on 2026-11-03. Nothing was worked around (operating rules).

`IrlosWeb/tasks/RECON.md` (a read-only reconnaissance of 2026-07-30) records that **the `GPU` SSH alias reaches the production GPU host**. Every fact below comes from that snapshot and is an **ASSUMPTION** until Step 01's preflight confirms it (Q-01, Q-02). Host names, addresses, user names and customer identities are deliberately omitted.

| Item | Assumed (RECON 2026-07-30) |
|---|---|
| OS | Ubuntu 22.04.5 LTS (glibc 2.35), systemd 249 |
| GPU | GeForce GTX 1650, TU117 die (Volta-generation NVENC), 4096 MiB VRAM, driver 580.126.18 (CUDA 13.0) |
| VRAM at idle | ~600 MiB used, ~3117 MiB free, ~380 MiB unattributed driver reserve. Measured with no tenant streaming; per-tenant cost under load is unmeasured |
| CPU / RAM / disk | 16-core Xeon, 62 GiB RAM, ~168 GB free on `/` |
| Installed | Python 3.10.12 and pip. **Absent:** Xvfb (apt candidate exists), Docker (no `docker0`), Rust toolchain, node |
| Customer workloads | Production customer stock-OBS instances (OBS 30.2.3 with obs-websocket 5.5.2) run as root under systemd. They share one Xorg `:0` with an XFCE session (hardware OpenGL through a dummy display plug, per the owner's notes), plus time-boxed x11vnc and per-tenant noalbs v2.16.1 |
| Security posture | No firewall rules. obs-websocket bound to all interfaces (5.5.2 has no bind-address setting). A human operator works on this host |
| Measurement caveats | Use `nvidia-smi pmon` and `nvidia-smi -q -d ENCODER_STATS` for encoder state, and `nvidia-smi pmon -s um` for per-process VRAM. `--query-compute-apps` misses graphics contexts, and **`--query-gpu=encoder.stats.sessionCount` must not be used** (owner instruction in `TASK_0_RECON.md`: unreliable on GeForce) |
| Unknown | Current state of all of the above; `ffmpeg` CLI and its NVENC support; CMake version; compiler and OpenSSL headers; NVIDIA Container Toolkit; the actual concurrent NVENC session limit |

What this means for the workflow (all handled in §9):
- `ssh GPU` is a production host shared with paying customers. All work is confined to `~/irlos-dev`, every GPU-using check runs behind the safety preflight, and the owner approves the first use and any testing windows.
- Local binaries (glibc 2.39) must not be copied to `ssh GPU` (glibc 2.35), so builds happen there.
- Stock Ubuntu 22.04 lacks the IrlosStudio build minima. The route is either an Ubuntu 24.04 dev container (the "dev box") or user-local dependencies, both owner-approved (D-09).
- With no firewall, every test listener binds `127.0.0.1` or a Unix socket. obs-websocket can only be tested inside a private network namespace.
- The GTX 1650 is **development hardware**. It cannot validate the 8-seat target, and every measurement taken on it is labelled as a development-hardware figure.

### 7. Supersession and revision note

**Supersession.** This file supersedes the following documents in `irlosd/prompts/`:
- `irlosd_alpha_master_specification.md` (the 16-step sequence);
- `step_1_rust_workspace_pruning.md` … `step_11_authoritative_state_reconciler.md`;
- `architecture_verification_steps_01_11.md`, for sequencing and step contracts;
- `development_specification_audit.md`, which had already declared itself superseded.

It also supersedes the Gemini-drafted step prompts. **None of these files was edited, moved or deleted.** They remain provenance. Their contracts were carried over, and every deliberate change is listed in Appendix C. An implementing agent does not need to read them. If one of them disagrees with this file, this file wins. If the code disagrees with this file, record the discrepancy (§2, rule 6).

**Revision 2 (2026-11-03).** Revision 1 (2026-11-02) was audited section by section against the drafting task, the current code, the IrlosStudio plugin and the planning documents, then revised. Revision 1 is preserved byte-for-byte at `irlosd/prompts/archive/2026-11-03/irlosd_roadmap_13_steps.md`. The 13-step grouping, the ordering decision (D-01), the dev root, the rsync workflow and the module names are unchanged. Decisions changed in revision 2, with reasons:

| Change | Revision 1 | Revision 2 | Reason |
|---|---|---|---|
| GPU encoder measurement | `--query-gpu=encoder.stats.sessionCount` in the preflight and checks | `nvidia-smi pmon` plus `-q -d ENCODER_STATS`; per-process VRAM via `pmon -s um` | Owner instruction in `TASK_0_RECON.md`; RECON showed the counters disagree on this GPU |
| Dev display numbers | `:90` and up | `:101`–`:199` | RECON verified this range is unused and avoids the VNC-implied displays |
| OBS v5 fallback isolation (D-22) | "loopback only" | Only inside a private network namespace (seat container or dev box) | obs-websocket has no bind-address setting (RECON), and the host has no firewall |
| IrlosStudio build route (D-09) | Route A or B, both left open | Route A is defined as a dev box container in which Studio, Xvfb, the fakes and a 24.04 build of irlosd run for all live checks. It is recommended but still owner-gated | A Studio built on 24.04 cannot run on the 22.04 host, so Route A needs a runtime as well as a build stage |
| Unattended Studio dialogs | Only the first-run wizard and exit confirmation were suppressed | Adds an inventory of the stream-start/stop error dialogs (unconditional modals), UI-blocked detection, a live check, and owner decision D-25 | `window-basic-main.cpp` shows `QMessageBox` on stream-start and encode errors and, while the window is visible, on connection failures. These block ICP's UI thread |
| Legacy deletion order | `noalbs.rs` deleted in 05, legacy chat in 09 | The whole legacy runtime, including `src/chat/` and `twitch_pubsub.rs`, is deleted in 05; Step 09 reuses code from Git history | `chat/twitch.rs` and `chat/kick.rs` import `ChatSender` from `noalbs.rs`, so 05 could not compile otherwise |
| SLS adapter | Cumulative drop counters; no `?reset` rule | Never send `?reset`; SLS interval counters default to `counterKind: "unknown"` (no rates) | RECON: `?reset` mutates shared state and the counters are interval values |
| Seat tool (Step 12) | `create`/`start`/`stop`/… | Command set and JSON shapes aligned with the IrlosWeb agent contract (`provision`, `suspend`, `destroy`, `status`, `capacity`, `control`) | Lets the planned GPU-host agent wrap `seatctl` one-to-one; the remaining contract points are Q-17 and D-06 |
| ICP numeric rule | Reject floats anywhere | Reject non-integers in documented integer fields; ignore unknown fields | `protocol.md` §4 |

Other revisions are corrections and added detail. The full per-step audit (keep/revise/rewrite) is in the revision summary given to the owner.

---

## Part II — Shared standards

### 8. Contract registry

Every shared name below is **defined in exactly one step** and spelled identically everywhere. "Extended by" means a later step adds variants or fields to a type it does not own; it never redefines it.

#### 8.1 Package, binary, paths and environment variables

| Item | Value | Defined in | Used by |
|---|---|---|---|
| Package and binary | `irlosd`, version `0.1.0-alpha`, edition 2024, **one package with no Cargo workspace**; library `src/lib.rs` + binary `src/main.rs` | 01 | all |
| Toolchain pin | `rust-toolchain.toml` with the exact stable version validated in Step 01 (≥ 1.88; the existing code uses let-chains, e.g. `src/switcher.rs:141`) | 01 | all |
| Dev-only binaries | `icp-probe` (02) and `ctl-probe` (03), in `src/bin/`, `required-features = ["dev-tools"]`; never in default builds or releases | 02, 03 | live checks |
| Subcommands | `serve --config <path>`, `status`, `command <verb> …`, `tui`, `check-config <path>`, `migrate-config <in> <out>`, `--version` | 01 (dispatch) | 05 `serve`, 06 `status`/`command`, 10 `tui` |
| Runtime dir | config `paths.runtimeDir`; production default `/run/irlos`, a seat-private tmpfs with mode `0700` owned by the seat UID (D-05). irlosd creates it if missing and refuses to run if it is group- or world-writable | 01 | 03–12 |
| ICP socket | `<runtimeDir>/obs-control.sock`; passed to IrlosStudio as env `IRLOS_ICP_SOCKET` | 03 (client), 04 (child env) | 05, 11 |
| Admin socket | `<runtimeDir>/irlosd.sock` | 06 | 10–13 |
| Runtime lock | `<runtimeDir>/irlosd.lock` (OS `flock`) | 04 | 05, 11 |
| Webhook socket | `<runtimeDir>/webhook/chat-webhook.sock`; `webhook/` is the only directory shared with the host edge proxy | 09 | 11, 12 |
| Session dir | config `paths.sessionDir`; production default `/var/lib/irlos/sessions/<sessionId>/` | 01 | 04, 09, 11, 12 |
| Studio generations | `<sessionDir>/studio/gen-<n>/` (contains `config/obs-studio/…`; `MANIFEST.json` written last) | 04 | 05, 11 |
| Studio templates | `templates/studio/<templateVersion>/` in the repository, embedded at build time | 04 | 11 |
| Secrets | `SecretRef` = JSON `{"file": "<absolute path>"}`; production directory `<sessionDir>/secrets/` (`0700`, files `0600`, owned by the seat UID) | 01 | 03, 04, 07, 09, 11, 12 |
| Chat replay ledger | `<sessionDir>/state/chat-replay.json` | 09 | 11 |
| Env vars | `IRLOS_ICP_SOCKET`: set by irlosd for its Studio child only, read by the plugin (03/04). `IRLOSD_ADMIN_SOCKET`: client override for `status`/`command`/`tui` (06). `IRLOSD_LOG`: log filter (01). Workflow only: `IRLOS_DEV_ROOT`, `IRLOS_LIVE_HOST` (default `GPU`), `IRLOS_LIVE_ROUTE` (`devbox`/`native`) (01) | as listed | — |
| Dev root on `ssh GPU` | `~/irlos-dev` (`IRLOS_DEV_ROOT`) | 01 | all |
| Dev box (Route A) | image `irlos-devbox:<studio-short-commit>`, container `irlos-dev-box`; the dev root is bind-mounted at the same absolute path | 01 | 01–10, 13 |
| Dev displays | `:101`–`:199`, chosen free by the preflight; private `Xauthority` in the run dir; never `:0` | 01 | 03–10 |
| Live-test entry point | `irlosd/live/live.sh` | 01 | all |
| Seat image and container | image `irlos-seat:<irlosd-version>-<studio-short-commit>`; container `irlos-seat-<sessionId>` | 11 | 12, 13 |
| Seat lifecycle tool | `irlosd/deploy/seatctl` | 12 | 13 |

Production code contains no hard-coded path other than these documented defaults. Dev sessions on `ssh GPU` set `paths.runtimeDir=$IRLOS_DEV_ROOT/run/<sessionId>` and `paths.sessionDir=$IRLOS_DEV_ROOT/sessions/<sessionId>`. Session IDs on `ssh GPU` stay short (`dev1`, `dev2`, …) so socket paths stay under the 108-byte `sun_path` limit.

#### 8.2 Module ownership

| Module (under `src/` unless shown) | Purpose | Defined in | Extended by |
|---|---|---|---|
| `main.rs`, `cli.rs` | entry point, `Cli`/`Command` dispatch | 01 | 05, 06, 10 |
| `domain.rs` | validated IDs, counters, epoch, `Sensitive<T>` | 01 | — |
| `error.rs` | `IrlosError`, public failure, dispositions | 01 | variants: 02, 03, 04, 05, 06, 07, 09 |
| `config.rs`, `config/legacy.rs` | canonical `IrlosdConfig`, `SecretRef`, load/validate, legacy NOALBS migration | 01 | blocks: 03–09 |
| `logging.rs` | `init_logging` and event field conventions | 01 | — |
| `broadcasting_software/icp/{mod,framing,messages,session}.rs` | ICP v1 | 02 | 03 |
| `broadcasting_software/{mod,types}.rs` | controller trait, common types, factory | 03 | 08 (repair mapping) |
| `broadcasting_software/unix_socket.rs` | `UnixSocketClient` | 03 | — |
| `broadcasting_software/obs_v5.rs` | `Obsv5` bridge (reworked legacy) | 03 | 08 |
| `supervisor/{mod,profile_gen,lock}.rs` | supervisor, profile generator, `RuntimeLock` (never also `src/supervisor.rs`) | 04 | — |
| `state.rs` | `SessionState` (private), `SessionSnapshot` DTOs | 05 | 07, 08, 09 |
| `reconciler/{mod,commands,policy}.rs` | actor, `SessionHandle`, command model, scene/output arbitration | 05 | 07 (`EngineInput`), 08 (`policy.rs`) |
| `daemon.rs` | `serve` composition root | 05 | 06, 07, 08, 09 |
| `admin/{mod,protocol,server,client}.rs` | admin IPC | 06 | 10 |
| `stream_servers/{mod,telemetry,sls,belabox}.rs` | telemetry | 07 | — |
| `switcher.rs` | `HysteresisPolicy` (pure; new file contents in 08) | 08 | — |
| `chat/{mod,gateway,commands,permissions,ledger,twitch,kick}.rs` | chat (new modules) | 09 | — |
| `tui/{mod,app,screens}.rs` | TUI | 10 | — |
| `bin/icp-probe.rs`, `bin/ctl-probe.rs` | dev-only live harnesses | 02, 03 | — |
| `tests/support/` | shared Rust fakes: mock ICP server, fake controller, fake supervisor, child helper | 02 | 03, 04 |
| `tests/live_*.rs` | `#[ignore]` live integration tests run by `live.sh` | 04 | 05 |
| `tests/architecture.rs` | source-tree test enforcing §8.11 | 05 | 06–10 |
| `templates/studio/` | versioned Studio profile and scene templates | 04 | — |
| `live/` | live-test workflow (§9) | 01 | every step |
| `deploy/` | dev box and seat images, seat init, compose templates, `seatctl` | 01 (dev box only), 11 | 12 |

Legacy retirement points:
- **Step 01:** the 11 unused providers, `web_server.rs`, `nginx/`, `launch.sh`, the startup GitHub check and dotenv loading.
- **Step 05:** the whole legacy runtime: `noalbs.rs`, `user_manager.rs`, `ws/`, `events.rs`, `twitch_pubsub.rs`, every file in `src/chat/`, the legacy `Switcher` runtime, `rust-i18n` with `locales/`, and the deprecated controller adapters.
- **Step 07:** the legacy typetag provider registry.

Step 09 writes new `src/chat/` modules and may reuse legacy code from Git history (`git --no-pager show dab2665:src/chat/twitch.rs`).

#### 8.3 Shared types and traits

| Name | Kind | Module | Defined in | Used by |
|---|---|---|---|---|
| `SessionId`, `TenantId`, `OutputId`, `MediaSourceId`, `RouteId`, `ProviderId`, `CommandId` | opaque validated IDs: ASCII `[A-Za-z0-9_-]`, 1–64 bytes, `FromStr` + `serde` | `domain` | 01 | all |
| `Revision`, `Generation` | checked `u64` counters; overflow is a terminal error, never wraps | `domain` | 01 | 03–09 |
| `DaemonEpoch` | random 128-bit value per `serve` start, from the OS RNG | `domain` | 01 | 05, 06, 09 |
| `Sensitive<T>` | wrapper whose `Debug`, `Display` and `Serialize` print `<redacted>` | `domain` | 01 | 02, 03, 04, 07, 09 |
| `SecretRef`, `SecretRef::resolve()` | file-backed secret; checks owner, mode ≤ `0600`, no symlink | `config` | 01 | 03, 04, 07, 09 |
| `IrlosdConfig` | canonical configuration root (§8.4) | `config` | 01 | all |
| `IrlosError`, `pub type Error = IrlosError` | error enum (§8.5) | `error` | 01 | all |
| `DaemonErrorCode`, `PublicFailure` | the only error form shown to clients | `error` | 01 | 05, 06, 09, 10 |
| `ExecutionCertainty`, `ConnectionDisposition`, `RetryDisposition`, `TelemetryFailureKind` | error metadata | `error` | 01 | 02, 03, 05, 07 |
| `read_frame`, `write_frame`, `MIN_PAYLOAD_SIZE`, `MAX_PAYLOAD_SIZE` | ICP framing | `broadcasting_software::icp::framing` | 02 | 03 |
| `IcpRequest`, `HelloResult`, `IcpOutputState`, `SceneSwitchResult`, `CurrentSceneResult`, `SceneListResult`, `OutputTransitionResult`, `OutputStateResult`, `StatsResult`, `IcpErrorCode`, `IcpErrorPayload`, `PROTOCOL_VERSION` | ICP v1.0 schemas | `…::icp::messages` | 02 | 03 |
| `IcpSession<S>` | handshaken, strictly serialized exchange over one stream | `…::icp::session` | 02 | 03 |
| `BroadcastingSoftwareLogic` | object-safe async controller trait | `broadcasting_software` | 03 | the Studio executor (05) only |
| `ControllerCapabilities`, `ObservationSelection`, `StudioObservation`, `FieldObservation<T>`, `OutputPhase`, `OutputObservation`, `StudioCommand`, `ControllerAck`, `ControllerEvent` | controller types | `broadcasting_software::types` | 03 | 05, 08 |
| `UnixSocketClient`, `Obsv5` | controller implementations | `…::unix_socket`, `…::obs_v5` | 03 | the factory only |
| `create_broadcasting_software`, `NetworkScope` | factory and its network-isolation input | `broadcasting_software` | 03 | 04 (config key), 05 |
| `RuntimeLock` | `flock` guard on `irlosd.lock` | `supervisor::lock` | 04 | 05 |
| `SupervisorHandle`, `ProcessObservation`, `ProcessLifecycleState` | process-owner API and facts | `supervisor` | 04 | 05 |
| `ProfileGenerator`, `GenerationManifest` | private Studio profile generation | `supervisor::profile_gen` | 04 | 05 |
| `SessionHandle` | **the only client API of the state owner** | `reconciler` | 05 | 06, 09 |
| `CommandEnvelope`, `Intent`, `OutputTarget`, `ModeRequest`, `CommandReceipt`, `ReceiptState`, `Capability`, `Principal`, `PrincipalKind`, `ChatProvider` (`Twitch`, `Kick`) | command model | `reconciler::commands` | 05 | 06, 09 |
| `SceneMode`, `HoldKind` | human scene-control state | `reconciler::policy` | 05 | 08, 09 |
| `EngineInput` | actor input enum | `reconciler` | 05 | extended by 07 (`Telemetry`) |
| `SessionSnapshot` (+ `OutputSnapshot`, `ControllerSnapshot`, `ProcessSnapshot`, `SceneSnapshot`) | redacted immutable snapshot on `watch<Arc<…>>` | `state` | 05 | 06, 09, 10; extended by 07, 08, 09 |
| `AdminRequest`, `AdminResponse`, `AdminNotification`, `AdminClient` | admin protocol and client | `admin` | 06 | 10, 12, 13 |
| `TelemetryProvider`, `TelemetryRunner`, `PollOutcome`, `StreamHealth`, `SampleQuality`, `FreshnessEvidence`, `CounterKind`, `ModemHealth` | telemetry | `stream_servers::telemetry` | 07 | 08, 09 |
| `HysteresisPolicy`, `PolicyInput`, `PolicyDecision`, `SceneProposal`, `Severity` | pure scene policy | `switcher` | 08 | the actor (wired in 08) |
| `StaleDataGuard`, `MediaRepairPolicy` | automation guards | `reconciler::policy` | 08 | — |
| `ChatAdapter`, `AuthenticatedChatMessage`, `ChatCommand`, `PermissionPolicy`, `ReplayLedger` | chat | `chat` | 09 | — |

#### 8.4 Configuration model

- **One private JSON file per session**, passed as `irlosd serve --config <path>`. Keys are camelCase, and the root carries `"irlosdConfigVersion": 1`.
- Unknown keys are rejected at the root and in every block marked *strict*. Duplicate legacy aliases are rejected.
- **Loading has no side effects.** It writes no files, creates no `.env`, changes no environment and makes no network or child-process calls.
- **File permissions.** `serve` requires a config owned by the current UID with mode ≤ `0640`; otherwise it exits 2. `check-config` validates content anywhere and *warns* about permissions. This lets it run on repository fixtures checked out as `0644`.
- `migrate-config` converts a legacy NOALBS JSON file in memory and writes the result atomically with mode `0600`. Its mapping table is in Step 01, and each owning step implements its block.
- **Secrets.** Secrets are only ever `SecretRef` (keys ending in `Secret`) or `Sensitive<String>` (secret-bearing URLs and publisher keys). They never appear in CLI arguments, logs or snapshots.
- **Reload.** Reload happens only through the `ReloadConfig` intent (CLI `irlosd command reload`, Steps 05–06; there is no SIGHUP).
  - It validates privately and applies a new config generation atomically.
  - It refuses output, credential or template changes while an output is active.

| Root key | Contents | Strict | Defined in |
|---|---|---|---|
| `irlosdConfigVersion` | integer, must be `1` | — | 01 |
| `session` | `sessionId`, `tenantId` | yes | 01 |
| `paths` | `runtimeDir`, `sessionDir`, `logDir` (nullable) | yes | 01 |
| `logging` | `format` (`json`/`text`; default `json` unless stdout is a TTY), `level` (default `info`) | yes | 01 |
| `software` | `type` (`unix_socket`/`obs_v5`) plus driver fields | yes | 03 |
| `studio` | supervisor, display, template, video/encoder, `outputMode`, `destinations[]`, `mediaSources[]`, `templateScenes` | yes | 04 |
| `engine` | reconciler timing and budgets | yes | 05 |
| `admin` | `operatorUids`, `readOnlyUids`, limits | yes | 06 |
| `telemetry` | `providers[]` | yes | 07 |
| `switcher` | hysteresis counts, scene mapping, thresholds | yes | 08 |
| `chat` | `commandPrefix`, `twitch`, `kick`, `commands`, `sceneShortcuts`, `permissions`, `rateLimits` | yes | 09 |

Steps 01–04 build with only the blocks defined so far. Each step that adds a block also adds:
- its defaults and validation;
- `check-config` coverage;
- a sanitized fixture under `tests/fixtures/config/`;
- its part of the legacy migration.

Fixtures contain only fake values.

#### 8.5 Error model

- **`IrlosError`** is defined with `thiserror` in Step 01, keeping the legacy constructors until their callers are deleted in Step 05. Later steps add only these variants:

  | Step | Variants |
  |---|---|
  | 02 | `Framing { kind, certainty }`, `Icp(IcpErrorPayload)`, `Handshake { reason }` |
  | 03 | `Controller { kind, certainty }`, `ObsV5(..)` |
  | 04 | `Process { kind, exit_code, signal }`, `Profile { reason }`, `LockHeld` |
  | 05 | `Engine { kind }` |
  | 06 | `Admin { kind }` |
  | 07 | `Telemetry { kind: TelemetryFailureKind }` |
  | 09 | `Chat { kind }`, `WebhookRejected { reason }` |

- **`DaemonErrorCode`** (snake_case on the wire): `config`, `invalid_input`, `unauthorized`, `wrong_session`, `conflict`, `busy`, `unavailable`, `unsupported`, `invalid_scene`, `timeout`, `outcome_unknown`, `protocol_blocked`, `process_spawn_failed`, `process_crashed`, `shutdown_timeout`, `recovery_exhausted`, `cleanup_incomplete`.
- **`PublicFailure { code, summary, retry_after_ms? }`** is the only error shape that reaches admin, TUI or chat. `summary` is an allowlisted fixed text, never remote text, a source chain, a path or a URL.
- **`ExecutionCertainty`**: `DefinitelyUnsent`, `Rejected` (the peer refused it), `MayHaveExecuted`.
- **Two independent dispositions:**
  - `ConnectionDisposition`: `Preserve` or `Drop`.
  - `RetryDisposition`: `RetryAfterObservation`, `RetryReadOnly`, `BlockUntilChange` or `DoNotReplay`.
- **`TelemetryFailureKind`**: `Transport`, `HttpAuth`, `HttpStatus`, `Parse`, `InvalidMeasurement`, `Oversize`. A telemetry failure is a quality outcome (Step 07), never a fake "Offline" or zero.
- **Rules:**
  - Never swallow an error or default to success.
  - An unknown wire code keeps its string and fails the operation.
  - An uncertain mutation stays uncertain until it is observed.
  - Each session keeps at most 100 sanitized failure records (Step 05).

#### 8.6 Lifecycle and supervision model

- **Ownership.** In production the chain is: container init (`tini`, PID 1) → seat init script → private display and `irlosd serve` → (supervisor) IrlosStudio in its own process group. Each concern has exactly one owner:

  | Concern | Owner |
  |---|---|
  | Studio's lifecycle | the supervisor |
  | The ICP or OBS connection and its reconnects | the controller driver task |
  | Per-output RTMP reconnects | libobs, through the profile's `[Output]` reconnect settings |
  | Session truth | the actor |

  Seat init stops the container when irlosd or the display exits. Nothing else launches Studio.
- **Startup** (Step 05):
  1. Parse and validate the config.
  2. Create or verify `<runtimeDir>` (mode `0700`; the plugin never creates its parent directory, `protocol.md` §2).
  3. Acquire `RuntimeLock`.
  4. Refuse to continue on an unknown live listener or a foreign process.
  5. Create a new `DaemonEpoch`. All observed facts start `Unknown`; desired outputs and recording start `Stopped`.
  6. Start the admin server (06), the telemetry runners (07) and chat (09).
  7. If `studio.startStudioOnBoot` is set, desired lifecycle becomes `Running` and the supervisor launches Studio.
  8. The controller connects and retries until the readiness check passes.

  No broadcast starts on boot.
- **Readiness.** Studio is ready when all three hold:
  - the process is `Running` in the current process generation;
  - ICP `hello` succeeded on a connection of that generation;
  - `get_current_scene` succeeded. Until Studio has loaded, the plugin answers `obs_unavailable` (`protocol.md` §9).
- **UI blocked.** IrlosStudio runs ICP operations on its UI thread, so a modal dialog blocks them (D-25). Step 05 detects this condition (`engine.studio_ui_blocked`) and reports it as `Degraded`. It never restarts Studio automatically unless the owner enables D-25's opt-in.
- **Shutdown** (SIGTERM, SIGINT or an authorized `Shutdown` intent):
  1. Latch `ShuttingDown` and reject new starts.
  2. Stop every managed output and recording through the executor, bounded.
  3. Wait a 500 ms grace period.
  4. The supervisor sends **SIGINT** to Studio's process group, waits `studio.childGraceMs` (5000), sends SIGKILL to the group and reaps it.
  5. Exit 0.

  The total is at most `engine.shutdownBudgetMs` (20000). If that is exceeded, report `CleanupIncomplete` and exit 3. Container stop grace must be at least 25 s.

  Why SIGINT: IrlosStudio handles SIGINT by closing its main window, which emits the frontend EXIT event so the plugin removes its socket. It has no SIGTERM handler (`UI/obs-app.cpp:2606-2681`; D-19). The generated `user.ini` sets `ConfirmOnExit=false` and outputs are stopped first, so no exit dialog appears.
- **Budgets** (each reset only by its own rule):

  | Budget | Value | Defined in |
  |---|---|---|
  | Supervisor relaunches | 5, with 1/2/4/8/16 s backoff | 04 |
  | Controller reconnect | 250 ms doubling to a 5 s cap | 03 |
  | Mutation retries | 5 attempts per domain revision, 500 ms → 5 s with ≤ 20% jitter | 05 |
  | Media repair | 3 attempts per source per 5 min | 08 |

- **Engine states:** `Initializing`, `Ready`, `Recovering`, `Degraded`, `ShuttingDown`, `Stopped`, `Failed`.
- **Never:**
  - adopt or kill an unknown PID;
  - unlink a live or unknown socket;
  - restart Studio only because control is unreachable (the default; see D-25);
  - let a client disconnect, a TUI exit or an SSH drop affect the session.

#### 8.7 Logging and event model

- Logging uses `tracing`. stdout gets JSON lines by default, or text when stdout is a TTY or `logging.format=text`. An optional rotating file goes in `paths.logDir`, with a bounded number of files. The filter is `logging.level`, overridden by `IRLOSD_LOG`.
- Every daemon event carries `event` (a dotted name), `session_id` and, from Step 05, `epoch`. Typed fields are added as needed, such as `output_id`, `revision`, `generation`, `duration_ms` and `error_code`.
- Events never contain secrets, URLs carrying keys, stream IDs, raw remote bodies or tokens.
- Text from Studio, platforms or chat is sanitized before it is logged: control characters stripped, length bounded.
- **Event names are an interface: live checks assert on them.** Each name is defined by one step, and that step's design specification lists the events it emits:

| Step | Event names |
|---|---|
| 01 | `daemon.starting`, `daemon.config_loaded`, `daemon.config_rejected`, `daemon.exit` |
| 02 | `icp.handshake_ok`, `icp.handshake_failed`, `icp.protocol_violation` |
| 03 | `controller.connected`, `controller.disconnected`, `controller.reconnect_scheduled`, `controller.request_failed`, `controller.peer_rejected` |
| 04 | `studio.profile_generated`, `studio.launching`, `studio.running`, `studio.ready`, `studio.exited`, `studio.restart_scheduled`, `studio.failed`, `studio.stopping`, `studio.stopped`, `studio.output` (debug; sanitized child output) |
| 05 | `engine.ready`, `engine.command_accepted`, `engine.command_rejected`, `engine.receipt_updated`, `engine.action_dispatched`, `engine.output_changed`, `engine.scene_changed`, `engine.degraded`, `engine.studio_ui_blocked`, `engine.config_reloaded`, `engine.shutdown_started`, `engine.shutdown_complete`, `engine.cleanup_incomplete` |
| 06 | `admin.listening`, `admin.client_connected`, `admin.client_rejected`, `admin.client_disconnected` |
| 07 | `telemetry.provider_started`, `telemetry.quality_changed`, `telemetry.poll_failed` (at most once per 30 s per identical failure) |
| 08 | `policy.proposal`, `policy.stale_data_brb`, `media.repair_proposed`, `media.repair_dispatched`, `media.repair_result` |
| 09 | `chat.adapter_state`, `chat.command_received`, `chat.command_rejected`, `chat.reply_failed`, `chat.webhook_rejected` |
| 11 | `seat.display_ready`, `seat.exit` (seat-init log lines in the same JSON shape) |
| 12 | `seatctl.action` (one audit line per lifecycle action) |

Step 10 (the TUI) is a client and writes nothing into the daemon's log stream.

#### 8.8 Path separation

1. **Media.** It stays in IrlosStudio. irlosd never reads, writes or proxies video, audio, SRT or RTMP bytes. It only writes URLs into private profiles (Step 04) and asks the controller for repairs (Step 08).
2. **Telemetry.** The flow is provider → `TelemetryRunner` → `EngineInput::Telemetry` → actor. Pollers never call the controller, change scenes or select video routes. A provider's priority never changes which video URL Studio pulls.
3. **Control.** Intents flow chat/admin/TUI/policy → actor → single Studio executor → `BroadcastingSoftwareLogic` → ICP or OBS v5. Supervisor commands come only from the actor.
4. Telemetry loss, chat loss and admin or TUI disconnects never stop outputs. Only these do:
   - an explicit Stop intent;
   - a confirmed revocation (the output was already observed stopped, Step 05);
   - shutdown.

#### 8.9 Per-session isolation

- **One session per process.** `serve` rejects a legacy multi-user `CONFIG_DIR` and any configuration naming more than one session.
- **Paths.** All runtime paths derive from `paths.*`.
- **No shared state.** No mutable global or static holds session data. Two engines must be constructible in one test process, and a test proves it (Step 05).
- **Telemetry.** Other publishers' records are discarded immediately (Step 07).
- **Admin.** Principals come from peer credentials and are scoped to the session (Step 06).
- **Chat.** Identities are scoped by `(provider, broadcaster_id, sender_id)` (Step 09).
- **Seats.** Each seat gets a separate UID, filesystem, sockets, display, network and resource limits (Steps 11–12).

#### 8.10 Protocol and engine constants

These are defaults; validated ranges are given in the defining step.

| Constant | Value | Defined in |
|---|---|---|
| ICP frame | u32 big-endian length prefix; payload 2..16384 bytes; strict UTF-8 JSON object; documented integer fields must be integers | 02 |
| ICP version | `protocol_version: 1` (document v1.0, Draft) | 02 |
| ICP client connect timeout | 2000 ms | 03 |
| ICP client request timeout | 5000 ms, and never below 5000 (`protocol.md` §9) | 03 |
| ICP reconnect | 250 ms doubling, 5000 ms cap, reset after a successful handshake, no jitter (`protocol.md` §9) | 03 |
| Plugin-side limits (not configurable by irlosd) | handshake 5 s, message 5 s, write 5 s, UI deadline 3 s, output starting guard 30 s, one client, at most 4 pending rejections (`src/icp/server.cpp:36-41`) | IrlosStudio |
| Admin frame | u32 big-endian length + UTF-8 JSON-RPC 2.0, ≤ 65536 bytes | 06 |
| Admin limits | 16 clients, 32 outstanding requests per client, 5 s read/write deadline, 128 queued notifications, ≤ 2 Hz snapshot notifications | 06 |
| Telemetry | poll 1000 ms, timeout 750 ms, decoded body ≤ 1 MiB; **never send sls `?reset`** | 07 |
| Freshness | `engine.telemetryFreshnessMs` 3000 | 05 (used by 08) |
| Hysteresis | degrade 3, recover 5, manual override 60 s | 08 |
| Engine | tick 1000 ms; Studio poll 1000 ms; observation freshness 5000 ms; confirmation 10000 ms; mutation attempts 5 (500 ms → 5000 ms); readiness warning 30000 ms; UI-blocked threshold 15000 ms; shutdown 20000 ms | 05 |
| Commands | queues 128 public + 128 internal; dedup 1024 entries per 10 min; submission lifetime 120 s | 05 |
| Supervisor | `maxRestarts` 5; backoff 1/2/4/8/16 s (cap 30 s); `childGraceMs` 5000; healthy reset after 60 s | 04 |
| Media repair | 1 in flight per source; observation 10 s; cooldown 30 s, then 60/120 s; at most 3 attempts per source per 5 min | 08 |
| Chat | command text ≤ 512 bytes; webhook body ≤ 64 KiB; mutations ≤ 10/min per sender and ≤ 60/min per session (burst 5); reads ≤ 20/min per sender; per-command cooldown 5000 ms; replay ledger 4096 entries per 10 min; Kick webhook max age 120 s and max skew 30 s | 09 |
| Limits | IDs ≤ 64 bytes; scene names ≤ 256 UTF-8 bytes; ≤ 8 managed outputs; ≤ 16 media sources; ≤ 16 providers | 01 / 05 |

#### 8.11 Bypass prohibitions (every step's acceptance criteria re-check these)

- **B1.** Only the actor (Step 05) mutates session state. Clients hold `SessionHandle`; producers send `EngineInput` facts.
- **B2.** Only the Studio executor (Step 05) holds an `Arc<dyn BroadcastingSoftwareLogic>`. No module outside `broadcasting_software` imports `icp::`, `obs_v5`, `unix_socket` or `obws`. `tests/architecture.rs` enforces this from Step 05 on.
- **B3.** Only the supervisor (Step 04) spawns, signals or reaps IrlosStudio.
- **B4.** Chat, admin, the TUI and the policy produce typed intents or proposals. None of them sends raw ICP/OBS requests, URLs, paths or unvalidated scene names.
- **B5.** No secret appears in logs, snapshots, CLI arguments, receipts, chat replies, test output or the run log.
- **B6.** No step assumes a single global session.
- **B7.** Every live check is registered with `live/live.sh` and runs under `check all`.

### 9. Development and live-testing workflow

#### 9.1 The two environments

| | Local machine (`/home/joystixxx`) | `ssh GPU` (live) |
|---|---|---|
| Purpose | Editing; `cargo fmt`, `clippy`, `build`, `test`; unit and integration tests with fakes | Real IrlosStudio, real NVENC on the GTX 1650, a private virtual display, real ICP sockets, realistic SRT feeds; containers (Steps 11–13) |
| Repos | `/home/joystixxx/irlosd`, `/home/joystixxx/IrlosStudio` | copies under `$IRLOS_DEV_ROOT/src/` |
| Builds | irlosd, for local tests only | irlosd, IrlosStudio, `icp_testserver`, dev box and seat images |

Local prerequisite: a user-local Rust toolchain via `rustup` (OA-0). Never copy a locally built binary to `ssh GPU`: local glibc 2.39 is newer than the server's assumed 2.35.

#### 9.2 Working directory on `ssh GPU`

Everything the agent creates on `ssh GPU` lives under `~/irlos-dev` (`IRLOS_DEV_ROOT`):

```
~/irlos-dev/
  src/irlosd/            rsync copy of the local irlosd tree (+ SYNC_MANIFEST.json)
  src/IrlosStudio/       rsync copy of the local IrlosStudio tree
  toolchain/             RUSTUP_HOME and CARGO_HOME (Route B, or host-side tools)
  prefix/                user-local build dependencies (Route B only)
  build/irlosd/          CARGO_TARGET_DIR (separate subdirectories per route)
  build/irlos-control/   standalone plugin build (icp_testserver)
  build/irlosstudio/     IrlosStudio CMake build
  opt/irlosstudio/       IrlosStudio install prefix (obs binary + plugins)
  run/<sessionId>/       runtime dirs: sockets, lock, Xauthority (short paths)
  run/pids/              process-group and container records of everything live.sh started
  sessions/<sessionId>/  session dirs: generated config, studio generations, test secrets
  seats/                 seat volumes (Steps 11–12)
  results/<run-id>/      check logs and summary.json
  media/                 synthetic test media and recordings (bounded; cleaned)
```

#### 9.3 Sync and builds

- **Sync** (`live/live.sh sync`).
  - rsync over `ssh GPU` copies both trees, honouring `live/rsync-exclude.txt`. It excludes `target/`, `.git/`, `live/results/`, build directories, `.env`, any `*.secret` file and local customer configs. The local `irlosd/.env` exists and must never leave the machine.
  - A local `SYNC_MANIFEST.json` records both commit hashes and a dirty flag.
  - Git is not used for sync: work under test is often uncommitted, and `ssh GPU` may not hold repository credentials (D-15).
- **Build route** (`IRLOS_LIVE_ROUTE`, decided by the owner, D-09).
  - **Route A — dev box (recommended).**
    - A long-lived container `irlos-dev-box` runs from `irlos-devbox:<commit>`, built from `deploy/devbox/Dockerfile`. That image is Ubuntu 24.04 with the CI dependency list from `.github/scripts/utils.zsh/setup_ubuntu`, Rust (pinned), Xvfb, xauth, `ffmpeg`, Python 3 and `jq`.
    - It runs with the NVIDIA runtime (`NVIDIA_DRIVER_CAPABILITIES=compute,video,utility`), as the dev user's UID/GID, with `$IRLOS_DEV_ROOT` bind-mounted at the same absolute path. It has its own network namespace with no published ports.
    - IrlosStudio, irlosd (built inside, against 24.04), Xvfb and all fakes for Steps 01–10 run inside it through `docker exec`. Studio and irlosd therefore share a PID namespace, so peer-PID checks hold.
    - It needs OA-6 only. Its build stage becomes the seat image's build stage in Step 11.
  - **Route B — native.**
    - irlosd builds on the host with a user-local rustup. IrlosStudio builds against user-local CMake ≥ 3.28, FFmpeg ≥ 6.1, nv-codec-headers ≥ 12 and nlohmann_json in `prefix/`, plus apt development packages (OA-7, OA-3).
    - Xvfb (OA-4) and the fakes run on the host.
    - OBS v5 bridge checks must then run inside `unshare --user --net` or are skipped (§9.8).
  - **Both routes:**
    - Populate `plugins/obs-websocket` and `plugins/obs-browser` (OA-8).
    - Configure with `-DENABLE_BROWSER=OFF` (no CEF; that this disables the submodule cleanly is Q-22) and `-DOBS_VERSION_OVERRIDE=30.2.3`. Keep obs-websocket enabled for the fallback.
    - Install to `opt/irlosstudio`.
    - Build the standalone plugin first (`cmake -S plugins/irlos-control -B build/irlos-control`). It provides `icp_testserver` and the ctest conformance wrapper.
- **irlosd build:** `cargo build --locked --release` with the pinned toolchain, in the dev box (Route A) or on the host (Route B). Native host builds need a C compiler, `pkg-config` and OpenSSL headers. If those are missing: OA-3, or the vendored OpenSSL feature the manifest already declares for musl (`Cargo.toml:43-44`).

#### 9.4 The live-test entry point

```
live/
  live.sh                 entry point; runs locally and drives ssh GPU
  lib/common.sh           ssh/devbox exec wrappers, run-id, redaction filter, process registry
  lib/remote_runner.sh    runs on ssh GPU (inside the dev box for Route A); executes checks
  lib/preflight.sh        read-only inventory and safety gate (§9.8)
  lib/admin.sh            status/command helpers (added in Step 06)
  checks/s01.sh … s13.sh  one file per step; each registers its checks
  fakes/                  test doubles (§9.11)
  fixtures/               sanitized configs and samples (fake values only)
  rsync-exclude.txt
  LIVE_RUNS.md            run log (committed)
  results/                fetched results (gitignored)
```

| Command | Effect |
|---|---|
| `live/live.sh preflight` | Read-only inventory and safety gate (§9.8); writes `preflight.json` |
| `live/live.sh sync` | §9.3 |
| `live/live.sh build [irlosd\|testserver\|studio\|devbox\|image\|all]` | Remote builds (`devbox`: Route A only; `image`: Step 11 on) |
| `live/live.sh check <sNN\|sNN-LK\|all> [--from sNN]` | Run checks on `ssh GPU`. `all` runs every registered check of every step in order; this is the regression gate |
| `live/live.sh fetch [<run-id>]` | rsync `results/<run-id>/` back to local `live/results/<run-id>/`, through the redaction filter |
| `live/live.sh clean` | Stop only the process groups and containers recorded in `run/pids/` (`irlos-dev-*`, `irlos-seat-dev*`); remove our runtime dirs. Never touches anything else |
| `live/live.sh all` | `preflight` → `sync` → `build all` → `check all` → `fetch` → `clean`. Exits nonzero on any FAIL or unexpected SKIP |

**Check contract**
- Each check is a shell function `check_sNN_lK`, registered with `register_check sNN-LK "<what it proves>" --needs <prereqs>`. Prereqs are `studio`, `display`, `nvenc`, `docker`, `rtmp_sink`, `srt_source`, `netns` or `chat_fake`.
- A check runs inside `$IRLOS_DEV_ROOT` and logs to `$RESULT_DIR/sNN-LK.log`.
- Return codes: `0` PASS, `1` FAIL, `77` SKIP. A SKIP prints the missing prerequisite or approval.
- A check cleans up what it started even on failure (`trap`).
- **SKIP is not PASS.** A step is done only if all its checks PASS. The only exceptions are checks tagged `--pending-hardware` (full-capacity validation) or `--manual` (needs real platforms); the summary reports them as such.
- **Run ID:** `YYYYMMDDTHHMMSSZ-<irlosd short commit>`.
- **`summary.json`:**

  ```
  { run_id, started_at, finished_at, irlosd_commit, irlosd_dirty, studio_commit, route,
    hardware_label: "ssh GPU — development hardware (GTX 1650)",
    customer_impact: { vram_delta_mb_outside_ours, foreign_processes_changed },
    checks: [ { id, title, status, duration_ms, log } ] }
  ```

#### 9.5 Run log

After every live run, append an entry to `live/LIVE_RUNS.md` containing:
- the date and time (UTC) and the run ID;
- both commits, dirty flags and the route;
- the step and unit;
- the exact command;
- PASS/FAIL/SKIP counts, and every non-PASS check with its reason;
- the customer-impact result (§9.8);
- at most 20 lines of relevant log excerpts, passed through the redaction filter;
- any measurement, labelled "development hardware (GTX 1650)".

Owner approvals (§9.9) are recorded here when given. Never paste secrets, IP addresses, host names or customer identities.

#### 9.6 Continuous live testing

For every step:
1. Work locally until one unit builds and passes `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --all-targets`.
2. At the unit's checkpoint, run `live/live.sh sync && live/live.sh build irlosd && live/live.sh check sNN`. Fix failures before starting the next unit.
3. At the end of the step, run `live/live.sh all`. **A step is incomplete until its checks pass on `ssh GPU` and every earlier step's checks still pass.**
4. Record each run (§9.5).

#### 9.7 Rules for the implementing agent on `ssh GPU`

- **R1.** Everything you create stays inside `~/irlos-dev`, or inside containers and images whose names start with `irlos-dev`/`irlos-seat-dev`/`irlos-devbox`.
- **R2.** No `sudo`, system package installs, driver, GPU or system-service changes, Docker daemon configuration or firewall changes without explicit owner approval in the conversation (§9.9).
- **R3.** Start processes and containers only through `live.sh` helpers that record them. Stop or restart only what you started. After each run, clean up: no test process, test broadcast, recording or container may be left running.
- **R4.** **No live broadcasts to real Twitch or Kick channels without owner approval.** By default outputs go to the local RTMP sink on `127.0.0.1`, inside the dev box or a test network, or to a recording file.
- **R5.** Never store real credentials in either repository. Test secrets are fake values in `sessions/<id>/secrets/` (`0600`). Real test-account tokens, once approved, are placed there by the owner, never committed and never echoed.
- **R6.** Never use display `:0` or any display, port, user, config directory, systemd unit, container or GPU context you did not create. Customer workloads and a human operator are on this machine (§6).
- **R7.** Outside the dev root, use only read-only commands: `nvidia-smi` queries, `ls`, `which`, `--version`, `ss -tln` (listing only).
- **R8.** Every listener you start binds `127.0.0.1` or a Unix socket, and Xvfb runs with `-nolisten tcp`. The host has no firewall, so anything bound to all interfaces is public. obs-websocket cannot be bound to loopback, so run it only inside the dev box or a private network namespace.
- **R9.** Label every measurement as a development-hardware figure.

#### 9.8 Safety preflight and customer-impact check

`live/live.sh preflight` is read-only and **always runs on the host**, outside any container: inside the dev box, `nvidia-smi` cannot see customer processes. It records:
- OS, glibc, CPU, RAM and free disk in the dev root;
- tool versions;
- `nvidia-smi --query-gpu=name,driver_version,memory.total,memory.used,memory.free,utilization.gpu`;
- `nvidia-smi -q -d ENCODER_STATS`;
- `nvidia-smi pmon -c 1 -s um`, giving per-process framebuffer and enc/dec usage;
- the X displays in use (`/tmp/.X11-unix`);
- listening ports (`ss -tln`, only to avoid them);
- the list of non-dev GPU processes, with PIDs and start times;
- any leftover processes or containers of ours.

**It never uses `--query-gpu=encoder.stats.sessionCount`** (owner instruction, `TASK_0_RECON.md`).

Gates before a check tagged `nvenc` or `studio`. The runner refuses (SKIP) in each of these cases:
- free VRAM is below `LIVE_MIN_FREE_VRAM_MB` (default 1024);
- `pmon` shows encoder usage that, plus this check's sessions, would exceed the measured session limit (Q-03);
- non-dev processes are using the GPU and the owner has not approved a testing window (OA-14).

After every GPU-using check, the runner repeats the snapshot and records `customer_impact`. Customer processes must be unchanged (same PIDs and start times), and VRAM outside our processes must not have moved by more than 64 MiB. A violation is a FAIL that stops the run and is reported to the owner. This mirrors the IrlosWeb rule "existing customer instances untouched, VRAM moved by exactly your delta".

Display numbers come from `:101`–`:199`, skipping any in use. Fakes bind `127.0.0.1` on high ports that the preflight confirms are free.

#### 9.9 Owner-approved actions

The agent must stop and ask before any of these. Approvals are recorded in `live/LIVE_RUNS.md`.

| ID | Action | First needed in |
|---|---|---|
| OA-0 | Local: install a Rust toolchain user-locally (`rustup`; network download) | 01 |
| OA-1 | First writes on `ssh GPU`: create `~/irlos-dev` and sync code there. This confirms the production host may be used for development at all (Q-02) | 01 |
| OA-2 | `ssh GPU`: user-local downloads into the dev root (rustup toolchain, CMake ≥ 3.28, static tools) | 01 |
| OA-3 | `ssh GPU`: system packages for native Rust builds (`build-essential`, `pkg-config`, `libssl-dev`), if missing (Route B) | 01 |
| OA-4 | `ssh GPU`: install `xvfb` and `xauth` on the host (Route B only; the dev box carries its own) | 01 |
| OA-5 | `ssh GPU`: `ffmpeg` CLI with `h264_nvenc` on the host (Route B only) | 01 |
| OA-6 | `ssh GPU`: install Docker Engine and the NVIDIA Container Toolkit, configure the NVIDIA runtime, and grant the dev user Docker access (Docker group access is root-equivalent; rootless Docker is an alternative). **Caution:** installing Docker adds bridges and iptables rules, including a FORWARD policy change, on a production host that today has no firewall | 01 (Route A), 11 |
| OA-7 | `ssh GPU`: IrlosStudio native build dependencies (apt Qt 6/dev packages plus user-local FFmpeg, nv-codec-headers, CMake) (Route B) | 01 |
| OA-8 | Local: populate IrlosStudio submodules `plugins/obs-websocket` and `plugins/obs-browser` (network fetch; changes that checkout's state) | 01 |
| OA-9 | Any change to IrlosStudio C++ or to `protocol.md` (D-04 companion, D-18, D-19, D-25) | 03, 04, 05, 08 |
| OA-10 | Real test chat accounts and channels and their tokens on `ssh GPU`; a Kick app, webhook subscription and edge endpoint | 09 |
| OA-11 | Any broadcast to a real Twitch or Kick channel | 09, 13 |
| OA-12 | Host users and groups, or Docker user-namespace remapping, for per-seat UIDs | 12 |
| OA-13 | Host firewall or network-policy changes for seat network isolation | 12 |
| OA-14 | A GPU testing window while customer workloads run, and long soaks (≥ 60 min) | 04, 12, 13 |
| OA-15 | Building or running a real `srt-live-server` in the dev root (the default is the fake stats server) | 07 (optional) |

#### 9.10 Hardware realism

- The GTX 1650 (4 GB VRAM, TU117 with Volta-generation NVENC) on `ssh GPU` is a **development and test GPU, not the production server**. Customers also use it (§6).
- GeForce drivers limit concurrent NVENC sessions. The limit for the installed driver must be **measured**, not assumed: Step 04 probes it with bounded test encodes, watching `pmon`. The IrlosWeb plan expects VRAM, not the session limit, to bind first ("3 to 5 tenants").
- **The 8-seats-per-server target cannot be validated on this machine.** Steps 12–13 test isolation and lifecycle with as many seats as the GPU supports at that time, alongside any customer usage. They record per-seat VRAM (`pmon -s um`), encoder use, CPU, RAM and frame statistics, and mark full-capacity validation **pending production hardware**.
- Every figure measured on `ssh GPU` is reported as "development hardware (GTX 1650)".

#### 9.11 Test doubles

| Double | Location | What it does | Created in |
|---|---|---|---|
| Mock ICP server (Rust) | `tests/support/mock_icp.rs` | Scriptable ICP v1.0 server over `UnixListener` or duplex streams, with fault injection | 02 |
| `icp_testserver` (C++) | IrlosStudio `plugins/irlos-control/src/testserver/` | The plugin's simulated Studio. Scenes `Live`/`BRB`/`Offline`/`Starting`; flags `--socket`, `--unavailable-ms`, `--start-fails`, `--ui-delay-ms`, `--ui-delay-running`, `--ui-delay-first`, `--reconnect-after-ms`, `--start-hangs`, `-v` | exists (built in 01) |
| ICP conformance suite | IrlosStudio `plugins/irlos-control/tests/icp_conformance.py` (`--mode stub\|live`, `--socket`, `--testserver`, `--allow-outputs`, `--wait-ready`, `--include-slow`) | Proves the plugin side | exists (run in 01) |
| Fake controller / fake supervisor (Rust) | `tests/support/` | Deterministic doubles for actor tests | 03 / 04 |
| SRT test source | `live/fakes/srt_source.sh` | ffmpeg `testsrc2` plus a tone as MPEG-TS from an SRT listener on `127.0.0.1` | 04 |
| RTMP sink | `live/fakes/rtmp_sink.sh` | `ffmpeg -listen 1` RTMP receiver on `127.0.0.1` writing a bounded file; reports received duration and gaps | 05 |
| Fake stats server | `live/fakes/fake_stats.py` (Python stdlib) | Serves `noalbs_sls_json` and `noalbs_belabox_json` payloads from scripted curves, with faults (timeout, 500, oversize, malformed, absent, foreign publishers) | 07 |
| Fake Twitch IRC | `live/fakes/fake_twitch_irc.py` | Loopback IRC with tags and capabilities; injects messages from scripted identities | 09 |
| Fake Kick | `live/fakes/fake_kick.py` | Sends webhooks to the webhook socket, signed with a test key via the `openssl` CLI; serves a fake public-key endpoint and chat-send API | 09 |
| Fault helpers | `live/lib/faults.sh` | `kill -9` the Studio child, SIGSTOP/SIGCONT Studio, stall the fake stats server, drop the SRT source, fill a test tmpfs | 13 (earlier steps add their own) |

---

## Part III — Step prompts

Each step below is self-contained when read together with Parts I and II, and its fifteen section headings are identical in every step.

Steps 02, 06, 10, 11 and 13 are close to the 2–3 page standard (about 1,650–2,000 words excluding code). Steps 01, 03, 04, 05, 07, 08, 09 and 12 run longer, up to about 2,700 words excluding code. Each of them either defines shared contracts (01, 03, 05) or carries source-verified details an implementing agent would otherwise have to rediscover: the IrlosStudio config keys and dialogs (04), the relay semantics (07, 08), the chat trust rules (09) and the agent contract shapes (12).

<!-- STEP 01 START -->
### Step 01 — Foundation and live-test workflow

#### 1. Title and metadata

| | |
|---|---|
| Step | 01 of 13: foundation (package, IDs, error model, configuration model, logging, CLI) and the local + `ssh GPU` live-test workflow |
| Depends on | none |
| Dependents | 02–13. Every step uses the workflow; 02–09 extend the error and config models |
| Absorbs old steps | 1 (workspace and pruning), 2 (error registry), plus the new development workflow |
| Code status at drafting | Not started (overview §3) |

#### 2. Objective

Turn the NOALBS 2.21.1 clone into the buildable single package `irlosd` 0.1.0-alpha. That means:
- validated domain IDs;
- the shared error model;
- the canonical configuration model with side-effect-free loading;
- structured logging;
- explicit subcommand dispatch;
- the legacy runtime disabled.

In the same step, build the development workflow every later step uses: sync to `ssh GPU`, remote builds (irlosd, `icp_testserver`, IrlosStudio) on the owner-chosen route, the `live/live.sh` entry point, the safety preflight, result collection and the run log.

#### 3. Why this step exists

Every later step needs four things this step provides:
- one name for the binary;
- one error type that carries execution certainty;
- one configuration model that never writes files as a side effect;
- a safe path to real hardware.

The legacy startup does all of these unsafely:
- it checks GitHub for releases (`src/main.rs:151`);
- it loads dotenv;
- conversions can write `config.json` or `.env`;
- Warp starts whenever `API_PORT` is set.

Live testing is continuous, so the workflow must exist before any feature code. It must also absorb the largest environment risks:
- `ssh GPU` is the production customer host;
- it has no Rust, display server or Docker;
- IrlosStudio needs newer build dependencies than Ubuntu 22.04 provides.

Each of those is handled through explicit owner-approved actions.

#### 4. Starting state

Confirm each item by reading the code.

- **`Cargo.toml`.** Package `noalbs` 2.21.1, edition 2024. Tokio features are `rt`, `rt-multi-thread`, `macros`, `signal`, `time` and `sync`. Dependencies include reqwest 0.13, obws 0.15, twitch-irc 6.1, warp 0.3, typetag, quick-xml 0.26, argon2 0.4, base64 0.21, async-recursion and rust-i18n 2.3. Vendored OpenSSL is declared for musl only (`:43-44`).
- **`src/lib.rs`.** Exports `broadcasting_software`, `chat`, `config`, `error`, `events`, `noalbs`, `state`, `stream_servers`, `switcher`, `twitch_pubsub`, `user_manager`, `web_server`, `ws`, plus `rust_i18n::i18n!("locales")`.
- **`src/main.rs`.** Runs the release check, dotenv and the logo. It then loads `CONFIG_DIR/*.json` or `config.json`, starts chat, optionally starts Warp, and waits on Ctrl-C. There is no argument parsing.
- **`src/error.rs`.** A 17-variant `Error`.
- **`src/stream_servers/`.** 14 providers; `obs.rs` is commented out of `mod.rs`.
- **`src/config.rs`.** Legacy `Config` with `user`, `switcher`, `software`, `chat`, `optionalScenes`, `optionalOptions`, `logToFile`.
- **Tests and tooling.** No `tests/` directory. Rust is absent locally (install: OA-0).
- **Live environment.** `ssh GPU` was unreachable at drafting.
- **Live checks.** None exist; this step creates them.

#### 5. Scope

**In scope**
- **Manifest.** Package rename and metadata, `rust-toolchain.toml`, Tokio `full` plus `test-util` (dev), `clap` 4 (derive), `tempfile` (dev), `nix` (signal, process, fs).
- **Pruning.** Remove the providers `xiu`, `nms`, `nimble`, `openrtmp`, `openirl`, `goirl`, `rist`, `mediamtx`, `nginx`, `irlhosting` and `websocket`, with their config variants. Also remove `stream_servers/obs.rs`, `web_server.rs` (with its `API_PORT` branch), `nginx/`, `launch.sh`, the release check, dotenv loading, and the deleted websocket's `stats_update_notifier()` select branch in `switcher.rs` (keep its 1 s tick).
- **New foundation modules.** `src/domain.rs`, `src/error.rs` (new `IrlosError`), `src/config.rs` + `src/config/legacy.rs`, `src/logging.rs`, `src/cli.rs`.
- **Legacy runtime disabled.** Dispatch never starts legacy chat, the switcher or Warp. `serve` returns a clear "available from Step 05" error.
- **Scaffolding.** Compile-safe empty modules: `broadcasting_software/icp/{mod,framing,messages,session}.rs`, `supervisor/{mod,profile_gen,lock}.rs`, `tui/mod.rs`. They export nothing that pretends to work.
- **Release workflow.** Never archive `.env`; ship `irlosd.example.json` with fake values.
- **Workflow.** The whole `live/` workflow (§9). On Route A, also `deploy/devbox/Dockerfile`.

**Out of scope**
- ICP code (Step 02).
- Controller changes (Step 03).
- Supervisor behaviour (Step 04).
- `serve`, and deletion of the legacy runtime and legacy chat (Step 05).
- Admin socket (Step 06).
- Telemetry rework (Step 07).
- Policy (Step 08).
- New chat (Step 09).
- TUI (Step 10).
- Seat images (Step 11).

Keep `async-recursion`, argon2/base64, `PubsubManager` and rust-i18n compiling until Step 05 deletes their callers.

#### 6. Design specification

**Domain (`src/domain.rs`)**

```rust
pub struct SessionId(String);   // one macro also defines TenantId, OutputId, MediaSourceId, RouteId, ProviderId, CommandId
impl FromStr for SessionId { type Err = IrlosError; /* 1..=64 bytes, [A-Za-z0-9_-] only */ }
pub struct Revision(u64);       // also Generation
impl Revision { pub fn next(self) -> Result<Self, IrlosError> /* checked_add; overflow = CounterExhausted */ }
pub struct DaemonEpoch([u8; 16]);   // OsRng; Display = 32 lowercase hex chars
pub struct Sensitive<T>(T);         // Debug/Display/Serialize print "<redacted>"; .expose() is explicit
```

Domain types import nothing from `config` or `broadcasting_software`.

**Errors (`src/error.rs`)**

This implements the §8.5 contract. Step 01 adds:
- `Io`, `Json`, `Config { field: String, reason: String }`, `InvalidId`, `Secret { field: String }`, `CounterExhausted`, `Logging`, `Unsupported { what: &'static str }`;
- the retained legacy constructors (`IoError`, `ConfigFileError`, `NoSourceFound`, `LangNotSupported`, `StatsPageNotAvailable`, `ObsV5Error`, …), shapes unchanged until Step 05.

```rust
pub enum ExecutionCertainty { DefinitelyUnsent, Rejected, MayHaveExecuted }
pub enum ConnectionDisposition { Preserve, Drop }
pub enum RetryDisposition { RetryAfterObservation, RetryReadOnly, BlockUntilChange, DoNotReplay }
pub struct PublicFailure { pub code: DaemonErrorCode, pub summary: &'static str, pub retry_after_ms: Option<u64> }
impl IrlosError { pub fn public(&self) -> PublicFailure; pub fn certainty(&self) -> ExecutionCertainty; }
```

`Display` never interpolates secret-bearing values.

**Configuration (`src/config.rs`)**

- Root type: `IrlosdConfig { irlosd_config_version: u32, session: SessionConfig, paths: PathsConfig, logging: LoggingConfig }`, annotated `#[serde(rename_all = "camelCase", deny_unknown_fields)]`.
- **Loading.** `load_config(path, mode: LoadMode::{Serve, Check})` reads one file, parses and validates it, and writes nothing. In `Serve` mode the file must be owned by the current UID with mode ≤ `0640`. In `Check` mode the same condition is only a warning.
- **`SecretRef { file: PathBuf }::resolve() -> Result<Sensitive<Vec<u8>>, IrlosError>`.** The path must be absolute and the file must:
  - not be a symlink (`O_NOFOLLOW`);
  - be owned by the current UID;
  - have mode ≤ `0600`;
  - be ≤ 64 KiB.

  The bytes are returned exactly. At most one trailing newline is stripped, with no other trimming or case change.
- **`write_private_atomic(path, bytes)`.** Same-directory exclusive temp file with mode `0600`, fsync, rename, then fsync of the directory.
- **Legacy (`config/legacy.rs`).** The existing NOALBS `Config` types move here unchanged. `migrate_legacy(&LegacyConfig, ids) -> Result<IrlosdConfig, IrlosError>` is fallible; an unknown removed provider name yields `Config`. The migration emits a list of warnings (no secret values). Each step implements its own rows:

| Legacy key | Canonical target | Implemented in |
|---|---|---|
| `user.{id,name,passwordHash}` | dropped. Identity comes from `migrate-config --session-id/--tenant-id`; names never become authority | 01 |
| `logToFile` | warning; set `paths.logDir` by hand | 01 |
| `software{type:"Obs",host,port,password,collections}` | `software.type = "unix_socket"` (ICP is primary). The legacy host, port and shared password are **not** carried over | 03 |
| `switcher.streamServers[]` | `telemetry.providers[]` (`SrtLiveServer`, `Belabox`; others rejected) | 07 |
| `switcher.{bitrateSwitcherEnabled, onlySwitchWhenStreaming, retryAttempts, triggers.{low,rtt,offline,rttOffline}, switchingScenes}` | `switcher.*` | 08 |
| `switcher.{instantlySwitchOnRecover, autoSwitchNotification}` | dropped, with a warning | 08 |
| `optionalScenes.{starting,ending,privacy,refresh}` | `studio.templateScenes` extras (04) and `chat.sceneShortcuts` (09); `refresh` is dropped (superseded by `!fix`) | 04, 09 |
| `optionalOptions.*` (transcoding checks, `offlineTimeout`, `recordWhileStreaming`, starting-scene automation) | dropped, with a warning (D-17) | 08 |
| `chat.{platform, prefix, commands.*.alias}` | `chat.commandPrefix`, `chat.commands.*.aliases` | 09 |
| `chat.{username, admins, ignoreUsers, permission rules}` | **not** carried as authority; warning: "re-provision moderators by stable user ID" | 09 |
| `chat.{enableAutoStopStreamOnHostOrRaid, announceRaidOnAutoStop, language}` | dropped, with a warning (D-17, D-12) | 09 |
| `chat.kick.{channelId, chatroomId, useIrlproxy}` | not usable for signed webhooks; warning (Q-08) | 09 |

**Logging (`src/logging.rs`).** `init_logging(&LoggingConfig, Option<&Path>) -> Result<LogGuard, IrlosError>` implements §8.7. Events emitted in this step: `daemon.starting`, `daemon.config_loaded`, `daemon.config_rejected` (field paths only) and `daemon.exit`.

**CLI (`src/cli.rs`, `clap` derive)**

```rust
pub enum Command {
    Serve { config: PathBuf },
    Status { socket: Option<PathBuf>, json: bool, health: bool },
    Command { socket: Option<PathBuf>, verb: Vec<String> },
    Tui { socket: Option<PathBuf> },
    CheckConfig { path: PathBuf },
    MigrateConfig { input: PathBuf, output: PathBuf, session_id: String, tenant_id: String },
}
```

- `--version` prints `irlosd 0.1.0-alpha (<git short hash or "unknown">)` before any config, log or network work. The hash is embedded by `build.rs` from `git rev-parse`; if Git is unavailable the build still succeeds.
- No arguments on a TTY means `tui`, which exits 2 until Step 10. With no arguments and no TTY, usage is printed and the exit code is 2.
- Unimplemented subcommands exit 2 with a clear message and **never fall through to starting an engine**.
- `check-config` exit codes: 0 valid, 2 invalid.

**Workflow (`live/`).** Create everything in §9.2–§9.5 with the names given there. `lib/common.sh` provides:
- `register_check`;
- `remote <cmd>` (ssh, plus `docker exec irlos-dev-box` on Route A);
- `start_tracked <name> -- <cmd>` (`setsid`, PGID recorded in `run/pids/`) and `stop_tracked`;
- `need <prereq>` (returns 77 with a reason);
- `redact`, a filter masking token-like strings, `streamid=`/`key=` values, `Authorization` values, IPv4/IPv6 addresses and host names.

All ssh calls use `ssh -o BatchMode=yes "${IRLOS_LIVE_HOST:-GPU}"`. The preflight in `lib/preflight.sh` implements §9.8 exactly.

#### 7. Implementation tasks

**Unit A — local toolchain and package (local only)**
1. Ask for OA-0. Install `rustup` user-locally and pin the toolchain in `rust-toolchain.toml`.
2. Rename the package, set the version, update the Tokio features, and add `clap`, `nix` and `tempfile` (dev). Defer ratatui and crossterm to Step 10.
3. Prune the providers, `web_server.rs`, `nginx/`, `launch.sh`, the release check and dotenv. Fix every reference, including the `config.json` sample and the README. Keep the build compiling after each removal.

**Unit B — workflow skeleton (checkpoint S01-L1, S01-L2)**

4. Present OA-1 to the owner (first use of the production host for development) and wait.
5. Write `live/` with `preflight`, `sync`, `check`, `fetch` and `clean`.
6. Run the preflight. Copy the observed environment, compared with §6, into `LIVE_RUNS.md`, and report every difference to the owner.

**Unit C — remote build route (checkpoint S01-L3)**

7. Present D-09 to the owner (Route A dev box or Route B native) with the OA list for each, and wait.
   - **Route A:** after OA-6, write `deploy/devbox/Dockerfile` and `live.sh build devbox`, then start `irlos-dev-box` (§9.3).
   - **Route B:** OA-2/OA-3, the user-local toolchain in `~/irlos-dev/toolchain`.
8. Build irlosd remotely with `cargo build --locked --release`.

**Unit D — foundation code (checkpoint S01-L4)**
9. Implement `domain.rs`, `error.rs` (keeping legacy variants), `config.rs` + `config/legacy.rs` (rows owned by Step 01), `logging.rs` and `cli.rs`.
10. Disable the legacy runtime in `main.rs`. Fix the release workflow and add `irlosd.example.json`.

**Unit E — ICP artefacts and IrlosStudio (checkpoints S01-L5, S01-L6, S01-L7)**

11. Build the standalone plugin (`build/irlos-control`), including `icp_testserver`, and run its ctest conformance wrapper in stub mode. Route B needs OA-2 for user-local CMake.
12. Ask for OA-8 (submodules). Build IrlosStudio into `opt/irlosstudio` on the chosen route (OA-7 for Route B).
13. Start a private Xvfb (`Xvfb :1NN -nolisten tcp -auth <run>/Xauthority`) on a free display in `:101`–`:199`.
14. Launch IrlosStudio:
    - environment: `IRLOS_ICP_SOCKET=$IRLOS_DEV_ROOT/run/s01/obs-control.sock`, plus a throwaway `XDG_CONFIG_HOME` under `run/s01/`;
    - flags: `--multi --disable-shutdown-check --disable-missing-files-check`.
15. Run the conformance suite in `--mode live` without `--allow-outputs`, then stop Studio with SIGINT.

#### 8. Error handling and recovery

- Config errors exit 2 before any side effect, listing field paths without values.
- A logging-initialization failure returns `IrlosError::Logging`, and the process exits 1.
- Counter overflow is `CounterExhausted`; from Step 05 on, callers treat it as a terminal engine failure.
- A missing or unsafe secret file yields `Secret { field }`. The error names the field, never the file's contents.
- In `live.sh`:
  - an ssh failure prints "ssh GPU unreachable" and exits 3, after at most one retry;
  - a remote build failure keeps its log in `results/`;
  - a failed preflight gate yields SKIP with a reason;
  - `clean` is safe to run repeatedly.

#### 9. Concurrency and lifecycle

- No daemon runs in this step.
- `--version` and `check-config` start no network tasks, children, log files or dotenv.
- Every process group and container the live helpers start is tracked and stopped on exit, including via `trap`.
- The manual Studio launch in task 14 is a test fixture only. It uses its own display, socket and config home, and is stopped with SIGINT, then SIGKILL after 10 s.
- Nothing here touches a broadcast or a customer process.

#### 10. Security and isolation

- Config and secret files are permission-checked as specified.
- `Sensitive` wraps every secret-bearing string.
- The production bypass of `ACCEPT_INVALID_CERTS` is removed; verified TLS stays the default.
- `rsync-exclude.txt` excludes `.env` and secret files. The local `irlosd/.env` exists and must never be synced, logged or copied into fixtures.
- Fixtures contain fake values only.
- `redact` runs on everything fetched into `LIVE_RUNS.md`.
- On `ssh GPU`, follow R1–R9:
  - listeners on `127.0.0.1` only;
  - Xvfb with `-nolisten tcp`;
  - never display `:0`.

#### 11. Local testing

- **`domain`:** ID bounds and characters; path-traversal strings rejected; checked counters; epoch uniqueness; `Sensitive` never prints its value.
- **`error`:** `public()` never contains private text (inject a canary secret and assert its absence); certainty and dispositions; every retained legacy constructor still compiles.
- **`config`:**
  - the canonical fixture loads;
  - an unknown key is rejected;
  - `Serve`-mode permission rejection vs `Check`-mode warning;
  - `SecretRef` symlink, mode and owner checks;
  - no file is created during load (snapshot a temp dir before and after);
  - a legacy fixture converts with the expected warnings;
  - an unknown provider is rejected.
- **CLI** (`tests/cli.rs`):
  - `--version` creates no files and makes no network calls (run with an unreadable HOME);
  - `check-config` exit codes;
  - `serve` refuses with "available from Step 05";
  - no arguments without a TTY exits 2.
- **Gates:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`.

#### 12. Live testing on `ssh GPU`

Register these in `live/checks/s01.sh`:

| Check | Proves | Expected |
|---|---|---|
| S01-L1 | The preflight runs read-only and records the full §9.8 inventory (pmon, ENCODER_STATS, displays, foreign GPU processes) | `preflight.json` written; no file outside `~/irlos-dev` changed |
| S01-L2 | The customer-impact check works: two snapshots around a no-op check compare equal | `customer_impact` all zero |
| S01-L3 | Sync, plus remote `cargo build --locked --release` on the chosen route with the pinned toolchain | binary in `build/irlosd/…/release/irlosd` |
| S01-L4 | `irlosd --version`, plus `check-config` on `live/fixtures/s01-valid.json` and `s01-invalid.json`, on the remote runtime | exits 0 / 0 (permission warning allowed) / 2; no log file created |
| S01-L5 | `icp_testserver` builds, and the conformance suite passes in stub mode against it | ctest PASS |
| S01-L6 | IrlosStudio installed in `opt/irlosstudio`; a private Xvfb starts on a free display in `:101`–`:199` | `obs --version` prints the IrlosStudio banner |
| S01-L7 | Real IrlosStudio plus plugin pass the conformance suite in live read-only mode; SIGINT exit removes the socket; customer impact is zero | PASS; socket path gone afterwards |

Cleanup: `live.sh clean` stops Xvfb and Studio and removes `run/s01`. Owner-approved actions: OA-0 to OA-8, as the route requires. S01-L6/L7 may stay SKIP only with the owner's written deferral, and in that case Step 03 cannot be declared done.

#### 13. Acceptance criteria and definition of done

1. `cargo fmt`, `clippy -D warnings` and `cargo test --all-targets` pass locally, with nonempty tests for `domain`, `error`, `config` and `cli`.
2. The package is `irlosd` 0.1.0-alpha, and the removed modules have no references (`rg` finds none).
3. No command can start legacy chat, the switcher, Warp or a child process, and `serve` refuses.
4. Config load and `check-config` create, modify or delete no file (proven by test).
5. The release workflow no longer copies `.env`.
6. `live/live.sh all` runs end to end, and S01-L1 to S01-L7 PASS on `ssh GPU` (or carry an explicit owner deferral as in §12).
7. `LIVE_RUNS.md` has an entry with the observed `ssh GPU` inventory, the chosen route and the approvals given, labelled development hardware.
8. Bypass rules B5–B7 hold.

#### 14. Handoff to later steps

- **Steps 02–09:** the `IrlosError` and `IrlosdConfig` frameworks, the domain IDs (including `CommandId`), `Sensitive` and the migration table to extend.
- **Step 03:** a working IrlosStudio build and `icp_testserver` on `ssh GPU`, the private-display procedure, the conformance suite and the chosen route.
- **Step 04:** the manual launch recipe, which it turns into the supervisor.
- **Step 05:** the disabled legacy runtime, to delete.
- **Every step:** `live.sh` (`register_check`, `remote`, `start_tracked`, `need`, `redact`, preflight and customer-impact gates) and the run log.

#### 15. Open questions and assumptions

- **Environment:** that `ssh GPU` is reachable with key-only authentication (Q-01) and may be used for development alongside customers (Q-02, OA-1, OA-14).
- **Build:**
  - the build route (D-09, Q-05);
  - the version override value `30.2.3` (D-09);
  - whether `ENABLE_BROWSER=OFF` cleanly disables the populated obs-browser submodule (Q-22).
- **Workflow choices** (rsync, `~/irlos-dev`, `live/` layout): D-15.
- **Toolchain:** D-16.
<!-- STEP 01 END -->

<!-- STEP 02 START -->
### Step 02 — ICP v1 protocol layer

#### 1. Title and metadata

| | |
|---|---|
| Step | 02 of 13: ICP v1 protocol layer (framing, typed messages, handshake, serialized session) |
| Depends on | 01 |
| Dependents | 03 (the only runtime consumer), 13 |
| Absorbs old steps | 3 (framing codec); 4 (schemas and handshake; core only; the old extension table becomes D-04) |
| Code status at drafting | irlosd: not started. IrlosStudio: server implemented at `564647bcb` (`plugins/irlos-control/src/icp/{framing,session,ops,errors,server}.cpp`); exercised by S01-L5/L7 |

#### 2. Objective

Implement the Rust side of ICP v1.0 exactly as `IrlosStudio/plugins/irlos-control/protocol.md` (Draft v1.0) and the plugin code define it:
- byte framing;
- strict JSON-object decoding;
- typed requests and results for `hello` plus the ten operations;
- the fourteen error codes with their closing rules;
- a handshaken, strictly serialized `IcpSession` over any async stream.

#### 3. Why this step exists

ICP is the primary control channel. The old Step 4 schema table disagrees with the plugin on almost every result:
- `get_current_scene` and the start/stop results;
- the output-state enum and the `get_stats` fields;
- the error codes and capability discovery.

A client built from that table would fail against the real server. This step makes the wire layer provably compatible, tested against the mock, `icp_testserver` and the real plugin, before any controller logic depends on it. Connecting, peer checks, timeouts and reconnects belong to Step 03.

#### 4. Starting state

- **Step 01 handoff:**
  - the `irlosd` package;
  - `IrlosError` with `ExecutionCertainty`, and `Sensitive`;
  - the empty modules `src/broadcasting_software/icp/{mod,framing,messages,session}.rs`;
  - `live.sh`, with S01-L1 to S01-L7 passing.
- `icp_testserver`, the conformance suite and an IrlosStudio build exist on `ssh GPU`, on the chosen route.
- Confirm with `live/live.sh check all`.
- Read `protocol.md` §§3–9 and `src/icp/{framing,session,ops,errors}.cpp` in IrlosStudio. They are the contract.

#### 5. Scope

**In scope:**
- `framing.rs`, `messages.rs`, `session.rs`;
- the ICP error variants on `IrlosError`;
- `tests/support/mock_icp.rs`;
- `tests/fixtures/icp/`;
- the dev-only `icp-probe` binary;
- live interoperability with `icp_testserver` and the real plugin.

**Out of scope:**
- `UnixStream` connect, peer credentials, timeouts, reconnect and the controller trait (Step 03).
- Mapping results into controller observations (Step 03).
- Any ICP v1.1 extension (D-04; gated unit in Step 03).
- Polling cadence (Step 05).

#### 6. Design specification

**Framing (`framing.rs`)**

```rust
pub const MIN_PAYLOAD_SIZE: usize = 2;
pub const MAX_PAYLOAD_SIZE: usize = 16_384;
pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<serde_json::Map<String, Value>, IrlosError>;
pub async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, obj: &serde_json::Map<String, Value>) -> Result<(), IrlosError>;
```

Reading:
1. Read the 4-byte big-endian length, distinguishing `CleanEof` (no header bytes) from `TruncatedHeader`.
2. Reject a length below 2 or above 16384 **before allocating** (`LengthOutOfRange`).
3. Read exactly N bytes (`TruncatedPayload`).
4. Check strict UTF-8 (`InvalidUtf8`).
5. Parse strict JSON per RFC 8259, with no trailing data, `NaN` or comments (`InvalidJson`).
6. Require a top-level object (`NotAnObject`).

Writing: serialize first, then check the size. An oversize payload fails with `Oversize`, certainty `DefinitelyUnsent`, and nothing is written. Otherwise write the header and payload, preferably as one buffer. A partial write error is `MayHaveExecuted`.

Errors are `IrlosError::Framing { kind: FramingErrorKind, certainty }`. After any framing error the stream must be dropped, which is Step 03's job; the codec itself never closes or retries.

**Messages (`messages.rs`)**

These are the shapes in `protocol.md` §§5–8. Result fields are required unless marked `?`.

| Request | Result |
|---|---|
| `{"op":"hello","protocol_version":1}` | `protocol_version` (= 1), `plugin_version`, `irlosstudio_version`, `obs_version` (informational strings) |
| `switch_scene {scene}` | `previous_scene` (may be `""` if no scene was loaded), `current_scene` |
| `get_current_scene` | `scene` (`""` if none is loaded) |
| `list_scenes` | `scenes: [string]` (in UI order), `current_scene` |
| `start_streaming`, `stop_streaming`, `start_recording`, `stop_recording` | `previous_state`, `state` (`IcpOutputState`); a no-op returns `previous_state == state` |
| `get_streaming_state`, `get_recording_state` | `state`, `duration_ms`, `last_error` (string; `""` when none) |
| `get_stats` | `timestamp_ms`, `active_fps_milli`, `average_frame_render_time_us`, `render_total_frames`, `render_skipped_frames`, `output_total_frames`, `output_skipped_frames`, `cpu_usage_permille`, `memory_usage_bytes`; `stream?` `{total_bytes, total_frames, dropped_frames, congestion_permille, connect_time_ms}` |

```rust
pub const PROTOCOL_VERSION: i64 = 1;
pub enum IcpOutputState { Inactive, Starting, Active, Reconnecting, Stopping }
pub enum IcpRequest { Hello, SwitchScene { scene: String }, GetCurrentScene, ListScenes, StartStreaming, StopStreaming,
    GetStreamingState, StartRecording, StopRecording, GetRecordingState, GetStats }
pub struct IcpErrorPayload { pub code: IcpErrorCode, pub message: Sensitive<String>, pub details: Option<Map<String, Value>> }
pub enum IcpErrorCode { VersionMismatch, ClientAlreadyAttached, MalformedHello, NotHandshaked, MalformedRequest,
    UnknownOperation, InternalError, ObsUnavailable, ObsTimeout, ResponseTooLarge, SceneNotFound, SceneNameTooLong,
    OutputBusy, OutputStartFailed, Unknown(String) }
```

Validation rules:
- **Envelope.** `ok: bool` plus exactly one of `result` (object) or `error` (object with string `code`, string `message` and optional object `details`). Both, neither, or wrong types is a `ProtocolViolation`, which means drop.
- **Numbers.** Every field `protocol.md` documents as an integer must be a JSON integer in i64 range. `1.0` and `1e0` are violations. **Unknown fields are ignored whatever their type** (§4), so a v1.x plugin can add fields.
- **Missing fields.** A missing required field is a violation, never a default.
- **Scene names** are compared byte-exact. Requests reject an empty name or one over 256 UTF-8 bytes **before sending**: an empty name is a *closing* `malformed_request` on the server.
- **Typed error details:**
  - `scene_not_found.details {requested, available?}` (`available` is omitted if it would not fit);
  - `obs_timeout.details.started` (`true` maps to `MayHaveExecuted`, `false` to `Rejected`);
  - `output_busy.details.state`;
  - `output_start_failed.details.last_error`.
- **Closing codes** (`errors.cpp: error_closes_connection`): `malformed_request`, `not_handshaked`, `malformed_hello`, `version_mismatch`, `client_already_attached`. All other codes preserve the connection.
- **Second client.** While another client is attached, the plugin answers a well-formed `hello` with `client_already_attached` *regardless of version* and closes (`session.cpp: reject_second_client`).

**Error mapping** (`impl IcpErrorPayload { fn classify(&self) -> (DaemonErrorCode, RetryDisposition, ConnectionDisposition) }`):

| ICP code | DaemonErrorCode | Retry | Connection |
|---|---|---|---|
| `obs_unavailable` | `unavailable` | `RetryAfterObservation` | Preserve |
| `obs_timeout` | `outcome_unknown` if `started`, else `timeout` | `RetryAfterObservation` | Preserve |
| `scene_not_found`, `scene_name_too_long` | `invalid_scene` | `BlockUntilChange` | Preserve |
| `output_busy` | `busy` | `RetryAfterObservation` | Preserve |
| `output_start_failed` | `unavailable` | `RetryAfterObservation` (bounded by Step 05) | Preserve |
| `response_too_large` | `protocol_blocked` | `BlockUntilChange` | Preserve |
| `unknown_operation` | `unsupported` | `BlockUntilChange` | Preserve |
| `internal_error`, `Unknown(_)` | `unavailable` | `RetryReadOnly` | Preserve |
| `client_already_attached` | `conflict` | `BlockUntilChange` | Drop |
| the other four closing codes | `protocol_blocked` | `BlockUntilChange` | Drop |

**Session (`session.rs`)**

```rust
pub struct IcpSession<S> { stream: S, hello: HelloResult, poisoned: bool }
impl<S: AsyncRead + AsyncWrite + Unpin + Send> IcpSession<S> {
    pub async fn handshake(stream: S) -> Result<Self, IrlosError>;          // sends hello first; validates version = 1
    pub async fn call<T: IcpResult>(&mut self, req: &IcpRequest) -> Result<T, IrlosError>;
    pub async fn probe_operation(&mut self, op: &str) -> Result<bool, IrlosError>; // for D-04 only: unknown_operation => false
    pub fn hello(&self) -> &HelloResult;
    pub fn is_poisoned(&self) -> bool;
}
```

- `call` takes `&mut self`, so one request is in flight at a time and there are no request IDs.
- These events poison the session:
  - any framing error or protocol violation;
  - a closing code;
  - cancellation mid-exchange, detected by a drop guard set between the start of the write and the end of the read.

  Every later call then fails with `Drop` disposition.
- `handshake` and `call` set no timeouts; Step 03 wraps every await in deadlines.
- **Events emitted:**
  - `icp.handshake_ok`, carrying `plugin_version`, `irlosstudio_version` and `obs_version` as fields. Behaviour never depends on them (§10).
  - `icp.handshake_failed`, carrying the code.
  - `icp.protocol_violation`, carrying the kind only, never payload bytes.

**Dev harness (`src/bin/icp-probe.rs`, feature `dev-tools`).** Usage: `icp-probe --socket <path> [--ops get_current_scene,list_scenes,…] [--allow-outputs]`. It prints one JSON line per result. `--allow-outputs` is required for any start/stop operation, and live checks pass it only against `icp_testserver`.

#### 7. Implementation tasks

**Unit A — framing (checkpoint S02-L1)**
1. Write `framing.rs` and its tests (§11).
2. Write `tests/support/mock_icp.rs`. It is scriptable per request (canned responses, delays, malformed frames, closing behaviour) and serves both `tokio::io::duplex` and a temp `UnixListener`.

**Unit B — messages and session (checkpoints S02-L2, S02-L3, S02-L4)**

3. Write `messages.rs` with exact serde shapes. Validate required fields strictly and ignore unknown ones.
4. Write `session.rs` and the error mapping, and add the `IrlosError::{Framing, Icp, Handshake}` variants.
5. Add golden fixtures under `tests/fixtures/icp/`, one success and one error file per operation, taken from `protocol.md`'s examples. Assert round trips.
6. Build `icp-probe` with `--features dev-tools` and run S02-L1…L4.

#### 8. Error handling and recovery

- Framing errors and violations drop and poison the session; Step 03 reconnects with backoff.
- `version_mismatch` blocks until the binary or config changes.
- `client_already_attached` is an ownership conflict. It is never retried in a tight loop; Step 03 backs off and reports it.
- Domain errors keep the session usable.
- An unknown error code is kept as a string and fails the call; it never counts as success.
- Never parse English `message` text to decide behaviour.

#### 9. Concurrency and lifecycle

- No tasks, timers or locks are created in this step.
- Each `IcpSession` is owned by exactly one task (Step 03's driver).
- **Cancellation safety:** if a `call` future is dropped after its write began, the session is poisoned and must be discarded. A late response must never be paired with a later request (`protocol.md` §6).
- Nothing here can start or stop an output by itself.

#### 10. Security and isolation

- Allocation is bounded at ≤ 16 KiB per frame, and serde's recursion limit is kept.
- Remote `message`/`details` are stored as `Sensitive` and never put into `PublicFailure` or logs. Logs carry only the code and the sanitized key names in `details`.
- No globals; one session object per connection.

#### 11. Local testing

**Framing**
- Boundary sizes: `{}` (2 bytes) and exactly 16384 bytes are accepted. Lengths 0, 1, 16385 and `u32::MAX` are rejected without allocation.
- Fragmentation: a header split 1+1+2; a payload fragmented at arbitrary offsets; two frames back to back.
- Truncation: clean EOF vs EOF after 1–3 header bytes; a truncated payload.
- Content: invalid UTF-8, trailing data, arrays and scalars, deep nesting.
- Writes: an oversize write produces zero bytes; a partial write failure reports `MayHaveExecuted`.

**Messages**
- Each operation's success fixture, and each error code.
- An unknown result field of any type is ignored.
- A documented integer given as `1.0` is rejected.
- A missing required field is rejected, as are both or neither of `result`/`error`.
- `obs_timeout` with `started` true and false.
- Exact Unicode scene names; empty and 257-byte names are rejected before sending.

**Session**
- `hello` goes first; a wrong `protocol_version` in the result is rejected.
- A closing code poisons the session, as does cancellation mid-read.
- A domain error preserves it.
- Calls are serialized.

Command: `cargo test --lib broadcasting_software::icp`.

#### 12. Live testing on `ssh GPU`

Run inside the dev box on Route A, on the host on Route B.

| Check | Proves | Expected |
|---|---|---|
| S02-L1 | `icp-probe` handshakes with `icp_testserver` | hello result printed; `icp.handshake_ok` |
| S02-L2 | Every read-only op parses against `icp_testserver`. `--unavailable-ms 3000` gives `obs_unavailable`, then success; `--ui-delay-ms 3500` gives `obs_timeout` with `started=false`; adding `--ui-delay-running` gives `started=true` | typed results; mapping matches §6 |
| S02-L3 | A second client gets `client_already_attached` and is closed while the first client keeps working. On the **test server only**, start/stop return `{previous_state, state}` transitions | PASS |
| S02-L4 | Against **real IrlosStudio** on a private display (Step 01 recipe): handshake and every read-only op parse; `list_scenes` matches the throwaway default collection | PASS; Studio stopped with SIGINT; customer impact zero |

Run `live/live.sh check s02`, then `check all`, then `clean`. No owner-approved actions beyond Step 01's.

#### 13. Acceptance criteria and definition of done

1. All local tests pass and are nonempty, and `clippy -D warnings` is clean.
2. Every schema in §6 matches `protocol.md` v1.0 and the plugin's `ops.cpp`/`session.cpp`. Any difference found is recorded under D-02.
3. Framing and session never allocate more than 16 KiB per frame and never pair a late response (proved by tests).
4. S02-L1 to S02-L4 PASS on `ssh GPU`, and all S01 checks still PASS.
5. `icp-probe` is excluded from default builds and release archives.
6. Outside `broadcasting_software::icp`, ICP types are used only by `broadcasting_software::unix_socket` (Step 03).

#### 14. Handoff to later steps

- **Step 03:** `IcpSession` (with `probe_operation`), `IcpRequest`, every result type, `IcpOutputState`, `classify()` and `mock_icp`.
- **Step 13:** `mock_icp` and `icp-probe`, for fault scenarios.

#### 15. Open questions and assumptions

- `protocol.md` is a Draft and may change incompatibly before irlosd v1 ships (`protocol.md` §10). The version pinned here is recorded in a test (D-02).
- Extensions are not part of this step (D-04).
- Whether the conformance suite passes is answered by S01-L5/L7 (Q-13).
<!-- STEP 02 END -->

<!-- STEP 03 START -->
### Step 03 — Broadcasting-controller interface (ICP client, OBS WebSocket v5 bridge, factory)

#### 1. Title and metadata

| | |
|---|---|
| Step | 03 of 13: one controller interface; `UnixSocketClient` (ICP, primary) and `Obsv5` (OBS WebSocket v5, explicitly configured fallback); the factory |
| Depends on | 01, 02 |
| Dependents | 04 (readiness probe), 05 (the Studio executor, the only runtime user), 08 (media-repair mapping), 13 |
| Absorbs old steps | 5 (Unix-socket controller and common boundary), 6 (OBS v5 bridge and dispatch) |
| Code status at drafting | Partial. Legacy trait `BroadcastingSoftwareLogic` (11 methods, including the lock-bound `info()`) and `Obsv5` exist; `noalbs.rs:59-64` always builds `Obsv5`. No ICP client or factory |

#### 2. Objective

Define the single, lock-free, object-safe controller interface the state owner uses to observe and command IrlosStudio, and implement it twice:
- **`UnixSocketClient`** over ICP v1.0. It owns the connection, applies deadlines and peer verification, and reconnects with ICP §9 backoff.
- **`Obsv5`**, a hardened OBS WebSocket v5 bridge with exact scene names and no shared-state writes.

Both are selected by `software.type` through one factory, never by automatic failover.

#### 3. Why this step exists

Every later effect on Studio goes through this interface; no other module may speak ICP or OBS (B2).

The legacy driver has five faults:
- it writes shared state;
- it matches scenes fuzzily and in lowercase;
- it sleeps 2 s while holding a lock;
- it panics on unknown sources;
- it leaves stale clients after a disconnect.

ICP has no events and allows one client, so the driver must own bounded, cancellation-safe exchanges that suit polling, and it must never replay an uncertain mutation.

#### 4. Starting state

- **Step 02 handoff:**
  - `IcpSession` (with `probe_operation`), the ICP types and `classify()`;
  - `mock_icp`, `icp-probe`;
  - the IrlosStudio build, `icp_testserver` and the private-display recipe on `ssh GPU`.
- The legacy `src/broadcasting_software/{mod.rs,obs_v5.rs}` is unchanged.
- Confirm with `live/live.sh check all` that S01 and S02 pass.

#### 5. Scope

**In scope:**
- `broadcasting_software/types.rs`, a rewritten `mod.rs` trait, `unix_socket.rs` and the `obs_v5.rs` rework;
- the `software` config block and its legacy migration;
- the factory;
- `tests/support/fake_controller.rs`;
- the dev-only `ctl-probe` binary;
- the owner-gated Unit D (IrlosStudio companion), only if approved.

**Out of scope:**
- Who calls the controller, polling cadence and retries (Step 05).
- Media-repair *policy* (Step 08); only the command mapping lives here.
- Generating obs-websocket config inside Studio profiles (Step 04).
- Named-output fanout objects in Studio (D-04 companion).
- Removing the deprecated legacy adapters (Step 05).

#### 6. Design specification

**Common types (`types.rs`).** These are independent of `state.rs` and `config`.

```rust
pub enum DriverKind { Icp, ObsV5 }
pub struct ControllerCapabilities { pub driver: DriverKind, pub scenes: bool, pub primary_output: Option<OutputId>,
    pub recording: bool, pub stats: bool, pub media_state: BTreeSet<MediaSourceId>, pub media_repair: BTreeSet<MediaSourceId>,
    pub named_outputs: BTreeSet<OutputId>, pub studio_versions: String /* sanitized, informational */ }
pub enum ObservationSelection { Scene, SceneList, Output(OutputId), Recording, Media(MediaSourceId), Stats }
pub struct FieldObservation<T> { pub value: Option<T>, pub valid: bool, pub connection_generation: Generation,
    pub process_generation: Generation, pub query_seq: u64, pub received_at: tokio::time::Instant }
pub enum OutputPhase { Inactive, Starting, Active, Reconnecting, Stopping }       // same meaning as IcpOutputState
pub struct OutputObservation { pub phase: OutputPhase, pub duration_ms: u64, pub last_error: Sensitive<String>,
    pub total_bytes: Option<u64>, pub dropped_frames: Option<u64>, pub congestion_permille: Option<u32> }
pub enum StudioObservation { Scene(FieldObservation<String>), SceneList(FieldObservation<Vec<String>>),
    Output(OutputId, FieldObservation<OutputObservation>), Recording(FieldObservation<OutputObservation>),
    Media(MediaSourceId, FieldObservation<MediaObservation>), Stats(FieldObservation<StudioStats>) }
pub enum StudioCommand { SetScene(String), SetOutput { id: OutputId, active: bool }, SetRecording(bool), RepairMedia(MediaSourceId) }
pub struct ControllerAck { pub command: StudioCommand, pub immediate: Option<StudioObservation> }
pub enum ControllerEvent { Connected { connection_generation: Generation, capabilities: ControllerCapabilities },
    Disconnected { connection_generation: Generation, reason: DaemonErrorCode }, Dirty { fields: Vec<ObservationSelection> } }
```

**Trait and factory (`mod.rs`)**

```rust
#[async_trait]
pub trait BroadcastingSoftwareLogic: Send + Sync {
    async fn capabilities(&self) -> Result<ControllerCapabilities, IrlosError>;
    async fn observe(&self, sel: ObservationSelection) -> Result<StudioObservation, IrlosError>;   // exactly one wire query
    async fn execute(&self, cmd: StudioCommand) -> Result<ControllerAck, IrlosError>;
    fn bind_process(&self, process_generation: Generation, expected_pid: Option<u32>);
    fn invalidate(&self);                                    // drop the current connection; the driver reconnects
}
pub enum NetworkScope { Host, Private }          // supplied from studio.networkNamespace (Step 04)
pub fn create_broadcasting_software(cfg: &BroadcastingSoftwareConfig, paths: &PathsConfig, primary_output: Option<OutputId>,
    network: NetworkScope, events: mpsc::Sender<ControllerEvent>) -> Result<Arc<dyn BroadcastingSoftwareLogic>, IrlosError>;
```

- The factory does no network I/O.
- `bind_process` is called by the Studio executor (Step 05) whenever the supervisor reports a new process generation. Connections and observations from an older generation are discarded.
- The deprecated legacy methods remain as `#[deprecated]` adapters returning `Err(Unsupported)`; Step 05 deletes them.

**Config (`software` block, defined here)**

```json
"software": { "type": "unix_socket", "socketPath": null, "connectTimeoutMs": 2000, "requestTimeoutMs": 5000,
              "reconnectBaseMs": 250, "reconnectCapMs": 5000, "expectedPeer": "child" }
"software": { "type": "obs_v5", "host": "127.0.0.1", "port": 4455,
              "passwordSecret": {"file": "/…/secrets/obs-websocket-password"}, "requestTimeoutMs": 5000 }
```

- `BroadcastingSoftwareConfig::{UnixSocket(UnixSocketConfig), ObsV5(ObsConfig)}`.
- `socketPath: null` means `<runtimeDir>/obs-control.sock`.
- `expectedPeer` takes one of two values:
  - **`child`** (the default) checks that the peer UID equals our UID **and** the peer PID equals the PID bound by `bind_process`. This is used under `serve` once a supervisor owns Studio.
  - **`same_uid`** checks the UID only. It is for `icp_testserver` and externally launched Studio in tests.
- Validation for `unix_socket`:
  - `requestTimeoutMs` ≥ 5000;
  - an absolute socket path shorter than 108 bytes.
- Validation for `obs_v5`:
  - `requestTimeoutMs` between 1000 and 60000;
  - `host` is a loopback literal;
  - `passwordSecret` is required. A test fixture may set `"auth": "none_test_only"`, but `check-config` rejects it unless `session.sessionId` starts with `dev`.
- **Isolation interlock.** obs-websocket binds every interface and has no bind-address setting (RECON).
  - The factory therefore takes `network: NetworkScope` (`Host` or `Private`) and refuses `ObsV5` with `Host`.
  - Step 04 adds the `studio.networkNamespace` key that supplies this value, together with the cross-block check in `check-config`.
  - Until then, `ctl-probe --network private|host` supplies it.
- **Legacy migration.** `software{type:"Obs"|"obs"|"websocket", host|ip, port, password}` always migrates to `{"type": "unix_socket"}`, because ICP is primary. The legacy host, port and password are never carried over, and a warning explains how to create a fresh per-seat password secret if the bridge is wanted. Giving both an alias and its canonical key is an error.

**`UnixSocketClient`**
- **Driver task.** One driver task owns the `UnixStream` and `IcpSession`. Public calls send `(request, oneshot)` over a bounded channel (capacity 32), and the driver serializes them.
- **Deadlines.** Connect gets `connectTimeoutMs`. `hello` and each request (write plus read) get `requestTimeoutMs`. If the deadline expires or the caller is dropped **after the write began**, the connection is invalidated and never reused, and the error carries `MayHaveExecuted`.
- **Peer verification** happens after connect and before `hello`:
  - Read `UnixStream::peer_cred()` and apply `expectedPeer`.
  - Confirm the path is `S_IFSOCK` with mode ≤ `0660`.
  - Confirm its parent directory is owned by us and not group- or world-writable.

  On failure, emit `controller.peer_rejected`, drop the connection and back off. The client never runs chmod, chown or unlink on the socket.
- **Reconnect** follows ICP §9: 250 ms doubling to a 5000 ms cap, reset after a successful handshake.
  - ENOENT/ECONNREFUSED while Studio starts is normal; keep retrying.
  - `version_mismatch` blocks reconnecting until `invalidate()` after a process change.
  - `client_already_attached` keeps backing off and reports `conflict`.
  - **Mutations are never replayed after a reconnect.**
- **Capabilities.** For ICP v1.0: `scenes`, `recording` and `stats` are true; `primary_output` is the factory argument; the media and named-output sets are empty. `RepairMedia` and named `SetOutput` therefore fail with `Unsupported` before any byte is sent.
- **Command mapping:**
  - `SetScene` → `switch_scene`; the returned `current_scene` must equal the target exactly.
  - `SetOutput{id == primary}` → `start_streaming` / `stop_streaming`.
  - `SetRecording` → `start_recording` / `stop_recording`.

  `{previous_state, state}` becomes `ControllerAck.immediate`.
- **Observation mapping:** `Scene` → `get_current_scene`, `SceneList` → `list_scenes`, `Output(primary)` → `get_streaming_state`, `Recording` → `get_recording_state`, `Stats` → `get_stats`. ICP has no events, so `Dirty` is emitted only after this driver's own mutations.
- **Events emitted:** `controller.connected`, `controller.disconnected`, `controller.reconnect_scheduled`, `controller.request_failed` (code and certainty only), `controller.peer_rejected`.

**`Obsv5` rework**
- Keep obws 0.15, with one connection task owning the obws client.
- Remove every `State`/`UserState` write, fuzzy and lowercased scene matching, the 2 s sleep under a lock, `event_stream.unwrap()`, the `unimplemented!` panic and the `fix()` that sends empty settings.
- **Mapping:**

  | Concern | OBS v5 requests |
  |---|---|
  | Scenes | `GetSceneList`, `GetCurrentProgramScene`, `SetCurrentProgramScene`, with exact membership checked first |
  | Primary output | `GetStreamStatus`, `StartStream`, `StopStream` |
  | Named outputs | `GetOutputStatus`, `StartOutput`, `StopOutput`, by the exact provisioned `outputName`, only if `GetOutputList` lists it |
  | Recording | `GetRecordStatus`, `StartRecord`, `StopRecord` |
  | Media state | `GetMediaInputStatus` for provisioned input names |
  | `RepairMedia` (mapping only; policy is Step 08) | For a *showing* `ffmpeg_source`: `TriggerMediaInputAction` with `OBS_WEBSOCKET_MEDIA_INPUT_ACTION_RESTART`. For a *hidden* network source: `SetInputSettings` with `overlay=true` and only `{"close_when_inactive": false, "restart_on_activate": false}`. The URL is never touched |
  | Stats | `GetStats` |

- Capabilities come from `GetVersion.availableRequests` plus the verified existence of the provisioned inputs and outputs.
- OBS events only produce `Dirty`.
- "Already started" and "already stopped" responses map to a confirming read, never to success.

**Dev harness (`src/bin/ctl-probe.rs`, feature `dev-tools`).** `ctl-probe --config <path> [--expected-pid N] [--observe <sel>…] [--execute <cmd>…]` builds a controller from config and prints JSON lines. It is the live harness until Step 06 and is never packaged.

**Unit D (owner-gated, D-04 / OA-9): IrlosStudio companion package.** Run it only if the owner approves; it is reviewed separately in the IrlosStudio repository. Contents:
- an additive ICP v1.1 revision of `protocol.md`: `get_capabilities`, `get_media_state`, `restart_media` (merged-settings update as above), `list_outputs`, `get_output_state`, `start_output`, `stop_output`;
- the corresponding Rust messages here, detected with `probe_operation`;
- the related Studio fixes D-18 (probe before stale-socket unlink), D-19 (SIGTERM handler) and D-25 (unattended dialog suppression).

Without approval, the alpha uses `outputMode=native_single` and repairs media only through `obs_v5`.

#### 7. Implementation tasks

**Unit A — types, trait, factory, fake (checkpoint S03-L1)**
1. Write `types.rs` and the new trait, keeping the deprecated adapters. Write `tests/support/fake_controller.rs`, which is scriptable with latencies, failures and observation sequences.
2. Add the `software` block, its migration row and the factory. Switch `noalbs.rs` construction to the factory; the legacy runtime stays disabled.
3. Write `ctl-probe`.

**Unit B — `UnixSocketClient` (checkpoints S03-L2, S03-L3)**

4. Implement the driver task, deadlines, peer verification and backoff, unit-tested against `mock_icp`.
5. Run S03-L2 against `icp_testserver` and S03-L3 against real Studio. Both use `expectedPeer: "same_uid"`, because no supervisor exists yet.

**Unit C — `Obsv5` rework (checkpoint S03-L4)**

6. Apply the hardening and mapping, with unit tests against an obws mock or recorded JSON exchanges.
7. Run S03-L4 **only inside a private network namespace**:
   - Route A: the dev box.
   - Route B: `unshare --user --net` with Studio, the bridge client and loopback inside. If unprivileged user namespaces are unavailable, SKIP with that reason.

   The check writes `plugin_config/obs-websocket/config.json` into the throwaway config home: server enabled, `first_load: false`, `alerts_enabled: false`, auth required, a fake per-run password file. Step 04 automates this.

**Unit D (gated)** — see §6. Stop and ask before starting it.

#### 8. Error handling and recovery

- Transport failures (EOF, ENOENT, ECONNREFUSED, deadline) map to `Controller{Unavailable|Timeout}`. The driver drops the connection and reconnects, and calls fail while disconnected.
- Domain errors follow Step 02's `classify()`.
- `PeerMismatch` and `AuthFailed` block reconnecting until `invalidate()`, logged at most once per 30 s.
- A failed observation marks only that field invalid; it never reports an output as stopped.
- Mutations that may have executed carry `MayHaveExecuted`, and the caller (Step 05) must observe before deciding anything.

#### 9. Concurrency and lifecycle

- One driver task per controller, and no `Mutex` is held across an `.await`.
- From Step 05 on, only the Studio executor holds the `Arc`; tests and `ctl-probe` hold it before then.
- Dropping the controller cancels the driver and closes the socket, but leaves Studio running.
- Disconnecting from ICP never affects a broadcast: the plugin keeps outputs running.
- No retries of mutations happen at this layer.

#### 10. Security and isolation

- The OBS password comes from a `SecretRef` and is never logged.
- Loopback client only, and the obs-websocket server runs only inside a private network namespace (the isolation interlock).
- ICP is protected by the peer UID/PID checks; the client never alters socket permissions.
- Remote error text is `Sensitive`.
- One controller per session; per-session socket paths.
- No raw-request API is exposed (B4).

#### 11. Local testing

- **Against `mock_icp`:**
  - `hello` and refusal;
  - `obs_unavailable` warm-up;
  - a slow response past the deadline (connection dropped, `MayHaveExecuted`);
  - a caller dropped mid-request does not shift later responses;
  - reconnect timing under paused time (250/500/1000/2000/4000/5000 ms, reset after handshake);
  - a second client gets `conflict`;
  - a peer UID or PID mismatch is rejected (through an injected credential source);
  - exact scene confirmation.
- **`Obsv5`:**
  - exact scene matching, never fuzzy;
  - a missing named output gives `Unsupported`;
  - an event-stream error reconnects without panicking;
  - the overlay repair payload contains only the two keys.
- **Config:**
  - both types;
  - alias conflicts;
  - a non-loopback host is rejected;
  - the factory rejects `obs_v5` with `NetworkScope::Host`;
  - timeout bounds;
  - the legacy `software` row.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S03-L1 | `ctl-probe` builds remotely; the factory selects the driver from fixtures; isolation-interlock rejection works | exit codes as expected |
| S03-L2 | `UnixSocketClient` vs `icp_testserver`: observe all fields; `SetScene("BRB")` confirmed; primary start/stop transitions observed; `--ui-delay-ms 3500` gives `timeout` with the connection kept; killing and restarting the test server reconnects within 5.5 s | PASS; `controller.connected`, `disconnected` and `reconnect_scheduled` logged |
| S03-L3 | `UnixSocketClient` vs real IrlosStudio: wait for readiness (scene query succeeds); observe all fields; switch scenes; start and stop **recording to a file** under `media/` | PASS; recording > 0 bytes, deleted at cleanup; customer impact zero |
| S03-L4 | `Obsv5` vs real IrlosStudio's obs-websocket inside a private network namespace, with auth: the same observations and scene switch; a wrong password is rejected; `ss -tln` on the host shows **no** new listener | PASS |

There is no streaming to any RTMP endpoint in this step.

#### 13. Acceptance criteria and definition of done

1. Local tests pass, including the paused-time backoff tests, and clippy is clean.
2. Both drivers implement the trait with identical result and uncertainty semantics.
3. No driver writes `State`. `rg` finds no fuzzy matching, `event_stream.unwrap()` or `unimplemented!` in `broadcasting_software/`.
4. Unsupported capabilities fail before any byte is sent.
5. S03-L1 to S03-L4 PASS (S03-L4 may SKIP on Route B only with the recorded namespace reason), and all S01–S02 checks still PASS.
6. Unit D is either not started, or approved and completed with its own conformance additions.

#### 14. Handoff to later steps

- **Step 04:** `observe(Scene)` for readiness, `bind_process`, and `NetworkScope`, which Step 04's `studio.networkNamespace` key must supply.
- **Step 05:** the trait, the factory, `ControllerEvent` and `fake_controller`; Step 05 deletes the deprecated adapters.
- **Step 08:** the `RepairMedia` mapping (via `obs_v5`; via ICP only after Unit D).
- **Later live checks:** `ctl-probe` remains a dev-only tool.

#### 15. Open questions and assumptions

- ICP timeouts and backoff follow `protocol.md` rather than the old 2 s values (D-03).
- The single-UID seat socket model and the peer-PID check (D-05).
- The IrlosStudio companion (D-04).
- OBS v5 isolation by network namespace (D-22).
- Which binary production controls (D-23, Q-10).
- That unprivileged user namespaces work on `ssh GPU` for Route B (Q-04).
<!-- STEP 03 END -->

<!-- STEP 04 START -->
### Step 04 — IrlosStudio supervision and profile generation

#### 1. Title and metadata

| | |
|---|---|
| Step | 04 of 13: single owner of the IrlosStudio process; private, versioned, generated profiles; bounded restart; clean stop |
| Depends on | 01, 02, 03 |
| Dependents | 05 (composes it), 11 (seat init), 12, 13 |
| Absorbs old steps | 10 (supervisor, profiles, destinations). Display and container provisioning move to 11; the fanout companion is D-04 |
| Code status at drafting | Not started. No process code; Tokio lacks `process`; `supervisor/` is empty scaffolding from Step 01 |

#### 2. Objective

Implement the supervisor that alone launches, watches, restarts and stops IrlosStudio. It must:
- generate a complete private profile and scene collection for one session;
- launch Studio on the selected private display with an allowlisted environment and `IRLOS_ICP_SOCKET`;
- publish process facts on a watch channel;
- stop Studio without modal dialogs or orphaned processes.

Also measure the NVENC session limit and record how Studio behaves when an unattended dialog appears.

#### 3. Why this step exists

Unattended operation needs a trustworthy process owner, and IrlosStudio has traps:
- **Signals.** It handles only SIGINT (`UI/obs-app.cpp:2606-2681`).
- **Config location.** It reads config from `$XDG_CONFIG_HOME/obs-studio/` and uses the newer `global.ini` + `user.ini` split (`UI/obs-app.cpp:686-760`).
- **Modal dialogs.** It shows modal dialogs at several points (table in §6). Each one blocks the UI thread, and with it every ICP operation.
- **Plugin socket handling.** The ICP plugin unlinks an existing socket at its path before binding, without checking for a listener (`protocol.md` §2).

Without a careful supervisor, a crash loop, two Studios on one socket, or a dialog nobody can click will each take a broadcast down.

#### 4. Starting state

- **Step 03 handoff:**
  - `UnixSocketClient` with `bind_process`, `observe(Scene)` and the readiness semantics;
  - `ctl-probe`;
  - the factory's `NetworkScope` input, which this step's `studio.networkNamespace` key supplies.
- **From Step 01:** IrlosStudio installed at `opt/irlosstudio` on `ssh GPU` (the chosen route) and the private-display recipe. S01–S03 pass.
- Empty `src/supervisor/{mod,profile_gen,lock}.rs`.
- Read before starting:
  - `UI/obs-app.cpp`: CLI flags at `:2721-2832`, `InitGlobalConfig`/`InitUserConfig` at `:686-760`, defaults at `:439-520`.
  - `UI/window-basic-main.cpp`: `FirstRun`/auto-config at `:2460-2475`, `LoadService` at `:1463`, `DisplayStreamStartError` at `:6248`, `StreamingStop` at `:6668`.
  - `UI/window-basic-main-outputs.cpp:500-760`: simple-output encoder selection.

#### 5. Scope

**In scope:**
- `RuntimeLock`;
- `SupervisorHandle` and the supervisor task;
- `ProfileGenerator`, with generation directories;
- `templates/studio/irlos-30.2.3-v1/`;
- the `studio` config block and the `optionalScenes` migration row;
- the launch specification and environment;
- stdout/stderr draining and the process-group stop sequence;
- the NVENC session-limit probe;
- the dialog-behaviour probe;
- `live/fakes/srt_source.sh`;
- `tests/support/{fake_supervisor,child_helper}.rs` and `tests/live_supervisor.rs`.

**Out of scope:**
- Deciding *when* Studio runs, and stopping outputs before shutdown (Step 05).
- UI-blocked detection and reaction (Step 05).
- Starting the display server. The dev display comes from `live.sh`; the production display from seat init (Step 11).
- Containers (Step 11).
- `managed_fanout`. Only `native_single` is generated; `managed_fanout` is rejected unless D-04 is implemented.

#### 6. Design specification

**Lock (`supervisor/lock.rs`)**

`RuntimeLock::acquire(runtime_dir) -> Result<RuntimeLock, IrlosError>`:
- Opens `<runtimeDir>/irlosd.lock` with `O_CLOEXEC` and mode `0600`, then takes a non-blocking `flock(LOCK_EX)`.
- A held lock yields `LockHeld`. A leftover file is harmless, because the flock itself is the lock.
- The guard lives until exit. Launching requires `&RuntimeLock`.

**Supervisor API (`supervisor/mod.rs`)**

```rust
pub enum ProcessLifecycleState { Stopped, Launching, Starting, Running, Crashed, Restarting, Failed, Stopping }
pub struct ProcessObservation { pub state: ProcessLifecycleState, pub pid: Option<u32>, pub process_generation: Generation,
    pub started_at: Option<SystemTime>, pub exited_at: Option<SystemTime>, pub exit_code: Option<i32>, pub signal: Option<i32>,
    pub launches_total: u32, pub restarts_remaining: u32, pub blocked_reason: Option<DaemonErrorCode> }
impl SupervisorHandle {
    pub async fn ensure_running(&self, lifecycle_revision: Revision) -> Result<(), IrlosError>;
    pub async fn ensure_stopped(&self, lifecycle_revision: Revision) -> Result<(), IrlosError>;  // SIGINT → grace → SIGKILL
    pub async fn reset_recovery(&self, lifecycle_revision: Revision) -> Result<(), IrlosError>;
    pub async fn report_ready(&self, process_generation: Generation);          // readiness fact, from the actor
    pub fn observe(&self) -> watch::Receiver<ProcessObservation>;
}
pub fn spawn_supervisor(cfg: &StudioConfig, paths: &PathsConfig, session: &SessionConfig, lock: &RuntimeLock)
    -> Result<SupervisorHandle, IrlosError>;
```

**Lifecycle**
- `ensure_running` is idempotent and never launches twice. Requests carry `lifecycle_revision`; any spawn or readiness from an older revision is fenced off.
- `Starting` becomes `Running` when `report_ready` arrives for the current generation. After 30 s without it, raise a warning flag, but **never kill a possibly broadcasting child for slowness**.
- A failure on first launch (bad binary, spawn error, invalid generation) latches `Failed` with no retry.
- After an owned child exits unexpectedly:
  - Relaunch after 1/2/4/8/16 s, up to `maxRestarts` attempts (cap 30 s for larger budgets; `0` disables relaunch).
  - Every attempt consumes budget.
  - 60 s of `Running` with readiness resets the consecutive budget, but not the lifetime total.
- `Failed` stays latched until `reset_recovery`. Each launch increments `process_generation`.

**Launch**

```text
<obsBinaryPath> --multi --disable-shutdown-check --disable-missing-files-check
                --profile <sessionId> --collection <sessionId> --scene <studio.templateScenes.brb>
```

- Exec the binary **directly**, with no wrapper script, so the ICP peer PID equals the child PID (Step 03 `expectedPeer: child`).
- Never pass `--startstreaming`, `--startrecording`, `--safe-mode` (it disables obs-websocket, `IRLOS_STRIP_PLAN.md`), shell strings or user flags.
- The child environment is an allowlist; nothing else is inherited:

  | Variable | Value |
  |---|---|
  | `HOME` | `<sessionDir>/studio/home` |
  | `XDG_CONFIG_HOME` | `<sessionDir>/studio/gen-<n>/config` |
  | `XDG_DATA_HOME` | `<sessionDir>/studio/home/.local/share` |
  | `XDG_RUNTIME_DIR` | `<runtimeDir>/studio-xdg` (`0700`) |
  | `DISPLAY` | `studio.display` |
  | `XAUTHORITY` | `studio.xauthorityFile` |
  | `QT_QPA_PLATFORM` | `xcb` |
  | `IRLOS_ICP_SOCKET` | `<runtimeDir>/obs-control.sock` |
  | `PATH` | a fixed vetted value |
  | `LANG` | `C.UTF-8` |
  | `LD_LIBRARY_PATH` | only if `studio.libraryPath` is set (Route B) |

- Before launch, create `<runtimeDir>` and `<runtimeDir>/studio-xdg` with mode `0700`. The plugin never creates the socket's parent directory.
- Start the child in its own process group (`process_group(0)`).
- Drain stdout and stderr continuously: 4 KiB line cap, 64 KiB queue, rate-limited, sanitized, logged as debug `studio.output`; dropped lines are counted.
- **Stop:**
  1. SIGINT to the group.
  2. Wait `childGraceMs`.
  3. SIGKILL the group.
  4. Reap exactly once.
  5. Remove a leftover socket only if our child is confirmed dead, the path is inside our runtime dir, and `lstat` shows a socket.

  A signal or wait error is `CleanupIncomplete`, never `Stopped`.
- **Events emitted:** `studio.profile_generated`, `studio.launching`, `studio.running`, `studio.ready`, `studio.exited` (code/signal), `studio.restart_scheduled`, `studio.failed`, `studio.stopping`, `studio.stopped`, `studio.output`.

**Profile generation (`supervisor/profile_gen.rs`)**

*Method.* The template is **derived from this exact IrlosStudio build**:
1. Launch it once in a throwaway `XDG_CONFIG_HOME`.
2. Create a profile and collection, then exit cleanly.
3. Commit the sanitized result as `templates/studio/irlos-30.2.3-v1/`, with placeholders.

Do not hand-write JSON from memory. `ProfileGenerator::generate(&StudioConfig, &SessionConfig) -> Result<GenerationManifest, IrlosError>` renders `<sessionDir>/studio/gen-<n>/config/obs-studio/` as follows:

| File | Required content (keys verified in source) |
|---|---|
| `global.ini` | As this build writes it, plus `[General] Pre31Migrated=true`, so the pre-31 migration (`obs-app.cpp:704-713`) never runs on start |
| `user.ini` | `[General] FirstRun=true` (suppresses the auto-config wizard, `window-basic-main.cpp:2462-2475`), `ConfirmOnExit=false`, `MaxLogs=10`; `[BasicWindow] RecordWhenStreaming=false`, `KeepRecordingWhenStreamStops=false`, `WarnBeforeStartingStream=false`, `WarnBeforeStoppingStream=false`, `WarnBeforeStoppingRecord=false` |
| `basic/profiles/<sessionId>/basic.ini` | `[General] Name=<sessionId>`. `[Output] Mode=Simple`, `Reconnect=true`, `RetryDelay=5`, `MaxRetries=5` (bounded native reconnect, old Step 10). `[SimpleOutput] StreamEncoder=nvenc` (selector names from `window-basic-main.hpp:65-76`, never internal IDs), `NVENCPreset2=<preset>`, `VBitrate`, `ABitrate`, `UseAdvanced=false`, `FilePath=<sessionDir>/recordings`. `[Video] BaseCX`, `BaseCY`, `OutputCX`, `OutputCY`, `FPSType=1`, `FPSInt=<fps>` |
| `basic/profiles/<sessionId>/service.json` | `{"type":"rtmp_custom","settings":{"server":…,"key":…,"use_auth":false}}` (shape from `LoadService`). Kick has no `rtmp-services` entry, so destinations always use `rtmp_custom` |
| `basic/scenes/<sessionId>.json` | Collection named `<sessionId>`, scenes from `studio.templateScenes`. `Live` and `Low` share one `ffmpeg_source` per media source with `is_local_file=false`, `input=<resolved URL>`, `close_when_inactive=false`, `restart_on_activate=false`, `log_changes=false`, `reconnect_delay_sec=5`, other options preserved exactly. `BRB` and `Offline` use only local assets (`image_source`); `Starting` is optional. No `browser_source` (CEF is not built) |
| `plugin_config/obs-websocket/config.json` | Only when `software.type=obs_v5` **and** `studio.networkNamespace="private"`: `{"server_enabled":true,"server_port":<port>,"auth_required":true,"server_password":<secret>,"first_load":false,"alerts_enabled":false}` |
| `MANIFEST.json` | Written last: template version, config generation, file hashes. No secrets |

Write and lifecycle rules:
- **Writes.** Directories `0700` and files `0600` from creation; exclusive temp file, fsync, rename. Symlinks are rejected, and the previous complete generation is kept as last-known-good.
- **Reuse.** A generation is reused across Studio relaunches, because Studio's own saves inside it are allowed. A new generation is created only when the config generation or template version changes, and it takes effect at the next launch. Never regenerate under a running child.
- **Secrets.** The stream key and the SRT input URL (whose stream id is the relay's only secret) are resolved here and **stored only in these private files**.
- **Studio's logs.** Studio writes its own logs under the generation (`obs-studio/logs/`), and those logs contain the media URL. Never fetch, print or archive them without `redact`.

**Unattended-dialog inventory.** Verify each row live (§12):

| Dialog | Trigger | Suppressed by |
|---|---|---|
| Auto-configuration wizard | first run | `user.ini` `FirstRun=true` |
| Safe-mode prompt | unclean previous exit | `--disable-shutdown-check` |
| Missing files | absent asset paths | `--disable-missing-files-check` |
| Exit confirmation | close while outputs are active | `ConfirmOnExit=false`, and outputs stopped first |
| "Already running" | second instance | `--multi` |
| **Stream start failed** | `DisplayStreamStartError()`, an unconditional `QMessageBox::critical` (`window-basic-main.cpp:6248-6256`) | **nothing (D-25)** |
| **Stream encode error / connection failure** | `StreamingStop()`: always on encode error; on connect failure whenever the main window is visible. Minimize-to-tray needs a system tray, which a private Xvfb lacks (`:2345-2347`, `:6732-6748`) | **nothing (D-25)** |
| **Recording path invalid** | `StartRecording()` → path check | **nothing (D-25)**; the generator guarantees a valid path |

**Encoder.**
- `StreamEncoder=nvenc` selects `obs_nvenc_h264_tex` if registered, else `ffmpeg_nvenc` (`window-basic-main-outputs.cpp:514-517`).
- Under software GL (Xvfb), the texture encoder reroutes to `obs_nvenc_h264_soft` (`plugins/obs-nvenc/nvenc.c:930-945`), which needs CUDA. Containers therefore need the `compute` capability.
- Confirm in Studio's log which encoder initialized.
- There is never a silent x264 fallback: a failed NVENC init fails the output start visibly.

**Config (`studio` block)**

```json
"studio": { "obsBinaryPath": "/…/opt/irlosstudio/bin/obs", "libraryPath": null,
  "displayProfile": "software_xvfb", "display": ":101", "xauthorityFile": "/…/run/dev1/Xauthority",
  "networkNamespace": "host", "startStudioOnBoot": true,
  "maxRestarts": 5, "initialBackoffMs": 1000, "maxBackoffMs": 30000, "childGraceMs": 5000,
  "templateVersion": "irlos-30.2.3-v1",
  "templateScenes": { "live": "Live", "low": "Low", "brb": "BRB", "offline": "Offline", "starting": "Starting" },
  "video": { "baseWidth": 1920, "baseHeight": 1080, "outputWidth": 1280, "outputHeight": 720, "fps": 30 },
  "encoder": { "streamEncoder": "nvenc", "videoBitrateKbps": 4500, "audioBitrateKbps": 160, "nvencPreset": "p5" },
  "outputMode": "native_single",
  "destinations": [ { "id": "main", "platform": "custom", "enabled": true, "server": "rtmp://127.0.0.1:19350/live",
                      "streamKeySecret": {"file": "/…/secrets/main-stream-key"}, "encoderGroup": "main" } ],
  "mediaSources": [ { "id": "ingest", "routeId": "main", "inputUrlSecret": {"file": "/…/secrets/ingest-url"} } ],
  "brbAsset": "/…/assets/brb.png" }
```

Validation:
- Paths are absolute.
- `displayProfile` is `software_xvfb` or `nvidia_xorg`.
- `networkNamespace` is `host` or `private` (declared by the deployment: seat init and the dev box set `private`). It maps to the controller factory's `NetworkScope`, and `check-config` rejects `software.type=obs_v5` together with `host`.
- `native_single` requires exactly one enabled destination.
- At most 8 destinations and 16 media sources.
- IDs are unique.
- Template scene names are unique, nonempty and ≤ 256 bytes.
- `streamEncoder` is in the selector allowlist, and `nvencPreset` is in the preset list read from this build.
- `childGraceMs` is between 1000 and 30000.

Migration: legacy `optionalScenes.{starting,ending,privacy}` become extra `templateScenes` entries.

#### 7. Implementation tasks

**Unit A — lock, template, generator (checkpoint S04-L1)**
1. Write `RuntimeLock`, the `studio` block and its migration row.
2. Derive `templates/studio/irlos-30.2.3-v1/` from the real build on `ssh GPU` (method above), sanitize it, and commit it.
3. Write `ProfileGenerator`, with golden-file tests.
4. Write `live/fakes/srt_source.sh`: ffmpeg `testsrc2` plus a sine tone, MPEG-TS on an SRT listener at `127.0.0.1:<free port>` with a fake `streamid`, tracked by `start_tracked`.

**Unit B — supervisor (checkpoints S04-L2, S04-L3)**

5. Write the supervisor task, launch, environment, draining, budgets and stop. Unit-test with `tests/support/child_helper.rs`, a test binary that obeys scripted signals and exits, never Studio flags passed to `/bin/sleep`.
6. Write `tests/live_supervisor.rs` (`#[ignore]`, configured by env). It generates a profile, launches the real Studio, gets readiness via `UnixSocketClient` (`expectedPeer: child`), crashes it, lets it restart, then stops it. The live check runs `cargo test --release --test live_supervisor -- --ignored`.

**Unit C — hardware and dialog probes (checkpoints S04-L4, S04-L5)**

7. Write `nvenc_probe`. With OA-14 where customers are present, it starts 1..8 concurrent 10 s `h264_nvenc` null encodes with an ffmpeg that has NVENC (checked by `ffmpeg -encoders`). The 8 is an upper probe bound, not a target. Each encode is watched with `nvidia-smi pmon`. Record the highest count that succeeded (development hardware) and stop them all.
8. Write `dialog_probe`. Configure a recording path the generator would reject: bypass validation via a test-only template. Send `start_recording` over ICP, and record what happens:
   - the ICP result (`obs_timeout` with `started=true` is expected);
   - whether later ICP calls still answer while the dialog is open (a nested event loop);
   - whether SIGINT still stops Studio.

   Write the observed behaviour into `LIVE_RUNS.md` and D-25. No network output is involved.

#### 8. Error handling and recovery

- Errors are `Process { kind: SpawnFailed | Exited | SignalFailed | WaitFailed }`, `Profile { reason }` and `LockHeld`.
- Initial failures latch `Failed`. Crashes consume budget, and an exhausted budget becomes `Failed` (`recovery_exhausted`).
- If the disk fills during generation, keep the last-known-good generation and refuse to launch a partial one.
- If the child survives SIGKILL handling, or unverified descendants remain, relaunching is blocked (`cleanup_incomplete`). Never kill a guessed PGID.
- If a live listener already answers on the socket path before launch, and the peer is not our child, the result is `conflict`.

#### 9. Concurrency and lifecycle

- One supervisor task `select!`s over commands, child exit, the backoff timer and readiness reports. It never blocks in an uncancellable wait.
- `kill_on_drop` covers only ordinary drops; container or cgroup teardown covers a SIGKILL of the daemon (Step 11).
- Stop always terminates the child, even if the controller is unreachable.
- The supervisor never decides to stop outputs; Step 05 stops them before calling `ensure_stopped`. On its own it relaunches only after a crash, when the broadcast is already gone.

#### 10. Security and isolation

- Private `0700`/`0600` files; secrets only in generated Studio files.
- An allowlisted environment, with no `LD_PRELOAD` and no `IRLOSD_*` passed to the child.
- Studio runs in its own process group. No `--safe-mode`.
- obs-websocket is enabled only in a private network namespace.
- Never touch display `:0`.
- Per-session paths come only from `paths.*` and `sessionId`.
- Socket cleanup happens only inside our runtime dir, and only after our child is confirmed dead.
- Studio's logs are treated as secret-bearing.

#### 11. Local testing

- Generator golden files: no secret in `MANIFEST.json`; permissions; symlink rejection; last-known-good kept after an injected write failure; reuse vs. new generation; obs-websocket config only with `private`.
- Budget sequences 5 and 0 under paused time.
- `ensure_running` is idempotent.
- Stop during backoff.
- A spawn during a stop race is killed at once.
- A late readiness report from an old generation is ignored.
- A helper that ignores SIGINT is SIGKILLed after the grace period.
- A log flood stays bounded.
- A second supervisor on the same runtime dir gets `LockHeld`.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S04-L1 | A generated profile loads in real Studio with no modal dialog: the Studio log shows no auto-config wizard or safe-mode prompt. Studio starts on `BRB`, and the ICP scene list equals `templateScenes` | readiness ≤ 60 s; `studio.ready` |
| S04-L2 | With `srt_source.sh` running, the `Live` scene shows the feed: `get_stats.render_total_frames` increases, and a 5 s recording is decodable (`ffprobe`). Studio's log names the NVENC encoder (`obs_nvenc_h264_soft` under Xvfb is expected). Without the source, Studio stays up on `BRB` | PASS |
| S04-L3 | `kill -9` of Studio's PID: relaunch after 1 s with a new process generation (the profile generation is reused). Exhausting `maxRestarts=2` (test config) latches `Failed`. `ensure_stopped` exits cleanly via SIGINT within the grace period, the socket is removed, and no process of the group remains | PASS |
| S04-L4 | NVENC session limit measured with `pmon` and recorded as development hardware | a number, or SKIP awaiting OA-14 |
| S04-L5 | Dialog probe outcome recorded (ICP behaviour while a modal is open; whether SIGINT still stops Studio) | outcome recorded; Studio stopped (SIGKILL fallback allowed) |

Every Studio-using check also asserts zero customer impact. Cleanup: stop the source, Studio and the display, and delete recordings and test generations. Owner-approved actions: OA-14 for S04-L4/L5 if customer workloads are present.

#### 13. Acceptance criteria and definition of done

1. Local tests pass and clippy is clean.
2. Only `supervisor/` spawns or signals Studio (`rg 'Command::new|kill\('` finds no other site).
3. No suppressible dialog appears in any live launch (S04-L1), and the unsuppressible ones are documented with observed behaviour (S04-L5).
4. Secrets appear only in generated Studio files with mode `0600`.
5. S04-L1 to S04-L3 PASS; S04-L4/L5 PASS or carry an explicit owner deferral; all earlier checks PASS.

#### 14. Handoff to later steps

- **Step 05:** `RuntimeLock`, `SupervisorHandle`, `ProcessObservation`, `ProfileGenerator`, `fake_supervisor` and the `studio` block (`destinations`, `mediaSources`, `templateScenes`, `networkNamespace`). `templateScenes.brb` is the safe scene.
- **Step 05 (D-25):** the S04-L5 dialog behaviour.
- **Step 11:** the launch spec, environment and template.
- **Steps 12–13:** the measured NVENC limit and `srt_source.sh`.

#### 15. Open questions and assumptions

- SIGINT stop, because Studio has no SIGTERM handler (D-19).
- Launch-flag and generated-key corrections: `--disable-missing-files-check`, `FirstRun`, `ConfirmOnExit`, `Pre31Migrated`, bounded `[Output]` reconnect (D-21).
- Unsuppressible error dialogs (D-25).
- The plugin's stale-socket unlink (D-18).
- Template scene names (D-14).
- The NVENC limit (Q-03).
- `nvidia_xorg` feasibility (D-07, Q-12).
- Simulcast needs D-04.
- Kick uses `rtmp_custom` (Q-15).
- That the NVENC soft path works under Xvfb inside a container (Q-12; verified by S04-L2).
<!-- STEP 04 END -->

<!-- STEP 05 START -->
### Step 05 — Authoritative state owner and the `serve` composition root

#### 1. Title and metadata

| | |
|---|---|
| Step | 05 of 13: one session actor holding desired and observed state; `SessionHandle`; reconciler and Studio executor; `irlosd serve` startup and shutdown; removal of the legacy runtime |
| Depends on | 01–04 |
| Dependents | 06, 07, 08, 09, 10 (all clients or fact producers), 11–13 |
| Absorbs old steps | 11 (core: state model, commands, outputs, lifecycle, persistence, shutdown). Old 11's automation (stale-data BRB, media repair) moves to 08 |
| Code status at drafting | Not started as specified. Legacy `Arc<RwLock<State>>` (`src/state.rs`) is written by `switcher.rs`, `obs_v5.rs`, `chat_handler.rs` and the `noalbs.rs` setters |

#### 2. Objective

Build the single authority for a session. A single-writer actor owns:
- **desired state**: human and automation intent;
- **observed state**: process, controller, outputs and scenes.

It accepts typed, authorized commands through `SessionHandle`, reconciles through one serialized Studio executor and the Step 04 supervisor, and publishes immutable, redacted `SessionSnapshot`s. `irlosd serve --config` composes everything and performs bounded startup and shutdown. The NOALBS runtime is deleted.

#### 3. Why this step exists

Every client (admin, TUI, chat) and every automation (telemetry, policy) needs one place to send intents and read truth. Without it, components drive Studio directly and overwrite each other, which is exactly the legacy NOALBS design. Building the state owner now, before telemetry and policy, gives every later step a real daemon to plug into and live-test through (D-01). It is also the step that must detect a Studio UI blocked by a modal dialog (D-25), because only the actor sees all observations together.

#### 4. Starting state

- **From Steps 01–04:**
  - `IrlosdConfig` with `session`, `paths`, `logging`, `software`, `studio`;
  - `create_broadcasting_software`, `ControllerEvent`, `fake_controller`, `ctl-probe`;
  - `RuntimeLock`, `SupervisorHandle`, `ProcessObservation`, `ProfileGenerator`, `fake_supervisor`, `child_helper`;
  - `tests/live_supervisor.rs`, `srt_source.sh`;
  - the S04-L5 dialog-probe outcome;
  - live checks S01–S04 passing.
- `serve` refuses to run, and the legacy runtime is disabled.
- The deprecated controller adapters and the legacy modules (`noalbs.rs`, `user_manager.rs`, `ws/`, `events.rs`, `twitch_pubsub.rs`, `src/chat/*`, the `Switcher` runtime) still compile.
- `chat/twitch.rs` and `chat/kick.rs` import `ChatSender` from `noalbs.rs`, so these modules can only be removed together.

#### 5. Scope

**In scope:**
- `state.rs`, `reconciler/{mod,commands,policy}.rs` (`policy.rs` covers scene modes and holds only), `daemon.rs`;
- the `engine` block;
- `serve`, with signal handling;
- UI-blocked detection;
- **deletion of the whole legacy runtime**: the modules above plus `rust-i18n` and `locales/`. Legacy `Triggers`, `SwitchingScenes` and `StreamServer` parsing types move into `config/legacy.rs` so migration keeps working;
- deletion of the deprecated controller adapters;
- `tests/architecture.rs`;
- `live/fakes/rtmp_sink.sh`;
- `tests/live_engine.rs` (`#[ignore]`);
- internal `ReloadConfig` handling.

**Out of scope:**
- The client socket transport (Step 06).
- Telemetry facts (Step 07 adds `EngineInput::Telemetry`).
- Hysteresis, stale-data BRB and media repair (Step 08). `RepairMedia` returns `unsupported` until then.
- The new chat gateway (Step 09), which reuses legacy chat code from Git history.

#### 6. Design specification

**Command model (`reconciler/commands.rs`)**

```rust
pub enum ChatProvider { Twitch, Kick }                                                 // reused by Step 09
pub enum PrincipalKind { LocalService, LocalOperator { uid: u32 }, LocalReadOnly { uid: u32 }, Chat { provider: ChatProvider, sender_id: String } }
pub struct Principal { kind: PrincipalKind, capabilities: BTreeSet<Capability> }      // constructible only in daemon/admin/chat
pub enum Capability { Read, Scene, Broadcast, MediaRepair, Record, Lifecycle, RecoveryReset, Hold, ReleaseAdminHold, Shutdown, Config }
pub struct CommandEnvelope { pub id: CommandId, pub epoch: DaemonEpoch, pub expected_revision: Option<Revision>,
                             pub admission_generation: Option<Generation>, pub intent: Intent }
pub enum OutputTarget { AllEnabled, One(OutputId) }
pub enum ModeRequest { Auto, Hold { kind: HoldKind, scene: Option<String> }, Release { kind: HoldKind } }
pub enum Intent { StartBroadcast { target: OutputTarget }, StopBroadcast { target: OutputTarget },
    SetScene { scene: String, ttl_secs: Option<u32> }, SetMode { mode: ModeRequest }, RepairMedia { source: MediaSourceId },
    SetRecording { active: bool }, SetLifecycle { running: bool }, ResetRecovery, ResetRepairBudget, ReloadConfig, Shutdown }
pub enum ReceiptState { Accepted, Applied, CompletedUnverified, OutcomeUnknown, Failed(PublicFailure), Superseded }
pub struct CommandReceipt { pub id: CommandId, pub state: ReceiptState, pub revision: Revision, pub per_output: BTreeMap<OutputId, ReceiptState> }
// reconciler/policy.rs (scene modes; Step 08 adds the automation guards to this file)
pub enum HoldKind { Manual, Administrative, External }
pub enum SceneMode { Auto, ManualOverride { target: String, until: Instant, owner: PrincipalKind },
                     Hold { kind: HoldKind, target: Option<String>, owner: PrincipalKind } }
```

**`SessionHandle` (`reconciler/mod.rs`)**

```rust
impl SessionHandle {
    pub async fn submit(&self, env: CommandEnvelope) -> Result<CommandReceipt, PublicFailure>;
    pub async fn command_status(&self, id: &CommandId) -> Result<CommandReceipt, PublicFailure>;
    pub fn snapshot(&self) -> Arc<SessionSnapshot>;
    pub fn subscribe(&self) -> watch::Receiver<Arc<SessionSnapshot>>;
}
pub enum EngineInput { Command(..), Urgent(..), Process(ProcessObservation), Controller(ControllerEvent),
                       Observation(StudioObservation), Completion(..), Tick }
```

`SessionHandle` values are minted only by `daemon.rs`, and from Step 06/09 on by the admin and chat constructors, each bound to one `Principal`. A request body can never assert identity or capabilities.

**Intent semantics** (carried over from old Step 11)
- **`SetScene`:**
  - The name must exist in the current-generation scene list, otherwise `invalid_scene`.
  - `ttl_secs: None` creates a `ManualOverride` lasting `switcher.manualOverrideSeconds`, or 60 s until Step 08 adds that key.
  - `Some(0)` creates a sticky `Hold { kind: Manual }`.
  - `Some(n)` overrides for *n* seconds (1–86400).
- **`SetMode`:**
  - `Auto` clears manual overrides and holds (Manual or External).
  - `Hold { kind: Administrative }` needs `Capability::Hold`, and only `Release { kind: Administrative }` with `ReleaseAdminHold` clears it.
  - `HoldKind::External` is created only internally, when a fresh, unexplained change to a non-automatic scene is observed.
- **`StartBroadcast`:**
  - It also requests lifecycle `Running`.
  - It preflights every targeted, enabled destination: secrets resolvable, capability present (`primary_output` for `native_single`). If any check fails, nothing starts.
  - `AllEnabled` means the enabled set captured at acceptance.
  - Results are per output; the aggregate `Partial` is derived, never stored.
- **`StopBroadcast`** leaves Studio running and the scene unchanged.
- **Admission:**
  - Every submission is rechecked against the principal's capabilities, the epoch and `expected_revision`.
  - Dedup is keyed by principal, `CommandId` and the normalized payload. A changed payload under the same ID is `conflict`. Entries are kept 1024 / 10 min; when the cache is full, new IDs get `busy` and nothing is evicted.
  - A submission lives at most 120 s and expires if still undispatched.
- **Priority:** `Shutdown` > lifecycle stop > output Stop > administrative hold > human intents in sequence > automation.
  - A Stop is latched in a retained urgent signal and raises an admission generation that rejects older queued Starts.
  - Queues are 128 public and 128 internal, and Stop and Shutdown never wait behind a full queue.

**State (`state.rs`, private to the actor)**

| Group | Contents |
|---|---|
| Identity and order | `SessionId`, `TenantId`, `DaemonEpoch`; a global desired revision plus independent revisions for scene, each output, recording, lifecycle and repair; config generation |
| Lifecycle | desired `Running`/`Stopped`; the latest `ProcessObservation` |
| Controller | `Disconnected`/`Connecting`/`Ready`/`Degraded{reason}`/`ProtocolBlocked`; connection generation; capabilities; UI-blocked episode |
| Outputs | `BTreeMap<OutputId, OutputState>`: desired `Active`/`Stopped`; observed `Unknown`/`Starting`/`Active`/`Reconnecting`/`Stopping`/`Stopped`/`Failed`; activation grant; pending action and certainty; retry budget; last failure |
| Recording | the same structure as one output |
| Scenes | effective desired scene and its origin; observed current scene (confirmed field plus age); scene list and its generation; `SceneMode` |
| Audit | the receipt cache; the last 100 sanitized failures |
| Engine | `Initializing`/`Ready`/`Recovering`/`Degraded`/`ShuttingDown`/`Stopped`/`Failed`; blocked conditions |

`SessionSnapshot` is a redacted DTO projection, serializable to JSON for Step 06. Its sub-objects are `ProcessSnapshot`, `ControllerSnapshot`, `OutputSnapshot` (one per output, plus recording) and `SceneSnapshot`:
- Every observed value carries its age and validity.
- It contains no config, secrets, URLs, keys or raw remote text.
- Aggregate output state (`AllActive`/`Partial`/`Stopped`/`Unknown`/`Degraded`) is computed at projection time.

**Reconciler and Studio executor**

The actor `select!`s over urgent signals, commands, process facts, controller events, completions and a 1000 ms tick that skips missed ticks. Each turn plans **one next action**, in this order:
1. Ensure the lifecycle.
2. Wait for readiness (§8.6), then call `supervisor.report_ready` and `controller.bind_process(generation, pid)`.
3. Refresh the scene list and capabilities.
4. Apply and confirm the scene.
5. Activate authorized outputs.

Stops preempt everything.

The **Studio executor** is a dedicated task holding the only `Arc<dyn BroadcastingSoftwareLogic>`. It runs one call at a time:
- Observation sweeps run every `studioPollMs` (1000 ms, per the `protocol.md` §9 recommendation) as round-robin per-field queries. Stop requests are prioritized.
- `ControllerAck.immediate` is not confirmation. A start stays pending until the output is observed `Active`. While the plugin reports `starting`, the start is left to the plugin's 30 s starting guard.
- If no state change is observed within `confirmationTimeoutMs`, the start becomes `OutcomeUnknown`; the executor then observes, and only then retries.
- Transient failures retry at 500 ms → 5 s with ≤ 20% jitter, at most 5 times per domain revision and process generation.
- `invalid_scene`, `unsupported` and `protocol_blocked` block until something relevant changes.

Revocation and recovery:
- **Revocation.** A fresh `Stopped`/`Failed` observation after `Active` that we did not cause revokes that output's grant. A new explicit Start is then required.
- **Studio crash.** The supervisor relaunches Studio, and desired outputs become `Stopped`, unless `resumeBroadcastAfterStudioCrash=true`. That flag allows resume only within the same epoch, for previously confirmed outputs.
- **Unreachable controller.** After 30 s the engine reports `Degraded`; it never restarts Studio for that reason alone.

**UI-blocked detection (D-25)**
- **Trigger.** A call returns `obs_timeout`, and every probe for `uiBlockedThresholdMs` (15000) also returns `obs_timeout` or times out.
- **Effect.** The engine logs `engine.studio_ui_blocked`, sets `Degraded{UiBlocked}` and fails or marks `OutcomeUnknown` on pending receipts.
- **Default reaction.** None beyond visibility. An authorized `SetLifecycle{running:false}` followed by `{running:true}` recovers, revoking output grants.
- **Opt-in.** With `restartStudioWhenUiBlocked=true` (owner decision D-25, default `false`), the actor itself performs that lifecycle cycle once per episode. Each cycle consumes the supervisor budget.

**`serve` (`daemon.rs`)**
- **Startup** follows §8.6 exactly. Studio launches only if `startStudioOnBoot` is set. The controller is built once, with `create_broadcasting_software(&cfg.software, &cfg.paths, primary_output, cfg.studio.network_scope(), events)`. `primary_output` is the single enabled destination of `native_single`.
- **Signals.** SIGTERM or SIGINT submits an internal `Shutdown` with the `LocalService` principal. A second signal during shutdown forces an immediate supervisor stop.
- **Shutdown** follows §8.6.

  | Exit code | Meaning |
  |---|---|
  | 0 | clean |
  | 1 | failure |
  | 2 | bad config |
  | 3 | `CleanupIncomplete` |

- **No daemonizing.** `serve` never daemonizes; systemd or container init runs it.
- **Rejected setups.** A legacy `CONFIG_DIR` is rejected.
- **Isolation test.** Two engines with different paths can run in one process.
- **Persistence.** Runtime state is memory-only, and a cold start always leaves desired outputs and recording `Stopped`. Durable writes are limited to config migrations and, from Step 09, the chat replay ledger.
- **`ReloadConfig`.** Re-reads the same path. While any output is active or pending, it rejects changes to `studio.destinations`, `studio.mediaSources`, the encoder, the template, secrets or `software`. Accepted changes apply under a new config generation (`engine.config_reloaded`).

**Config (`engine` block)**

| Key | Default |
|---|---|
| `tickMs` | 1000 |
| `studioPollMs` | 1000 |
| `studioObservationFreshnessMs` | 5000 |
| `telemetryFreshnessMs` | 3000 |
| `confirmationTimeoutMs` | 10000 |
| `retryBaseMs` | 500 |
| `retryCapMs` | 5000 |
| `mutationAttempts` | 5 |
| `readinessWarningMs` | 30000 |
| `uiBlockedThresholdMs` | 15000 |
| `restartStudioWhenUiBlocked` | `false` |
| `shutdownBudgetMs` | 20000 |
| `resumeBroadcastAfterStudioCrash` | `false` |
| `autoRepair` | `true` (used by 08) |

Validation:
- Timings are 1–60000 ms, except confirmation, freshness, readiness, UI-blocked and shutdown, which may be up to 300000.
- Each freshness value must exceed its poll interval.
- `shutdownBudgetMs` must exceed `childGraceMs + requestTimeoutMs + 500`.

**Events emitted:** `engine.ready`, `engine.command_accepted`, `engine.command_rejected`, `engine.receipt_updated`, `engine.action_dispatched`, `engine.output_changed`, `engine.scene_changed`, `engine.degraded`, `engine.studio_ui_blocked`, `engine.config_reloaded`, `engine.shutdown_started`, `engine.shutdown_complete`, `engine.cleanup_incomplete`.

#### 7. Implementation tasks

**Unit A — model and pure reducers (local)**
1. Write `state.rs`, `reconciler/commands.rs` and the scene-mode parts of `reconciler/policy.rs` as pure functions `(state, input) → (state, planned action)`, with exhaustive paused-time tests.

**Unit B — executor and composition (checkpoints S05-L1, S05-L2)**

2. Write the executor, the actor loop, UI-blocked detection and `daemon.rs::serve`.
3. Write `live/fakes/rtmp_sink.sh`. It is an `ffmpeg -listen 1` RTMP receiver at `rtmp://127.0.0.1:<free port>/live/<fake key>` that writes `media/sink-<run>.flv` (bounded to 120 s) and reports received seconds and the largest gap.
4. Write `tests/live_engine.rs` (`#[ignore]`). It builds the engine through `daemon::build_engine(config)` and drives it **through `SessionHandle`** with a `LocalOperator` principal. That is the only way intents are submitted until Step 06.

**Unit C — legacy removal and enforcement (checkpoints S05-L3, S05-L4)**

5. Delete the legacy runtime (§5) and the deprecated adapters, along with dependencies left unused: warp, argon2, base64, async-recursion if unused, tokio-tungstenite, twitch-irc (re-added in 09), rust-i18n and dotenv. Keep migration working through `config/legacy.rs`.
6. Add `tests/architecture.rs`. It scans `src/**/*.rs` as text and fails if:
   - a file outside `broadcasting_software/` mentions `icp::`, `unix_socket`, `obs_v5` or `obws`;
   - a file other than `reconciler/` or `daemon.rs` names `dyn BroadcastingSoftwareLogic`;
   - `Command::new` appears outside `supervisor/`;
   - a `static` holds a `Mutex` or `RwLock`.

#### 8. Error handling and recovery

- Errors are `Engine { kind: ChannelClosed | ShutdownTimeout | CleanupIncomplete | Exhausted }`.
- An unexpected exit of a critical task (executor, supervisor or a runner) triggers a bounded failure shutdown; no replacement writer is ever started.
- Clients only ever see `PublicFailure`.
- Retries are bounded (§8.6), and `OutcomeUnknown` blocks replay until an observation resolves it.
- An invalid config at startup exits 2 before the lock is taken.

#### 9. Concurrency and lifecycle

- The actor never awaits I/O and never holds a guard across an `.await`. The executor does all controller I/O; the supervisor does all process I/O.
- Snapshots go out over `watch`, so a slow reader cannot block the actor.
- Dropping a handle never cancels accepted intent.
- **Nothing in this step stops a running output except** an explicit Stop, a confirmed revocation (the output was already observed stopped), shutdown, or the opt-in UI-blocked cycle (off by default).
- No broadcast starts on boot.

#### 10. Security and isolation

- Capabilities attach to principals minted inside the daemon.
- Snapshots are redacted. A test injects canary secrets into the config and asserts they are absent from serialized snapshots and logs.
- No globals, and a two-engine isolation test proves it.
- The lock and runtime directory are checked before any child is spawned.

#### 11. Local testing

Use `fake_controller`, `fake_supervisor` and paused time. Cover:
- **Startup:** facts start Unknown and outputs Stopped.
- **Admission:** capability rejection; dedup replay, conflict, full cache (`busy`) and expiry; revision independence (a scene flood does not reset output retries); a Stop overtakes queued Starts.
- **Starts:** a failed preflight starts nothing; a pending Start is first observed `Stopped`; `MayHaveExecuted` is observed before any retry.
- **Revocation and crashes:** revocation on an unexplained `Stopped`; crash with no resume and with opt-in resume; a 30 s unreachable controller gives `Degraded` without a restart.
- **UI blocked:** detection at the threshold, and the opt-in cycle runs once per episode.
- **Scenes:** an external scene gives `Hold{External}`; TTL semantics (`None`/`0`/`n`); an administrative hold resists Manual release.
- **Reload and shutdown:** reload is rejected while active; shutdown ordering and the 20 s budget, including `CleanupIncomplete`.
- **Robustness:** a critical-task panic; two engines in one process; canary secrets absent; `tests/architecture.rs`.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S05-L1 | `irlosd serve --config <s05-dev1.json>` with `startStudioOnBoot=true` reaches `engine.ready` with Studio on `BRB`. SIGTERM shuts down in < 20 s with `engine.shutdown_complete` and exit 0; no Studio process or socket remains | PASS |
| S05-L2 | `live_engine` against real Studio, `srt_source.sh` and `rtmp_sink.sh`: `SetScene Live` Applied; `StartBroadcast AllEnabled` Applied with the output observed Active and the sink receiving ≥ 20 s; `StopBroadcast` Applied and the sink ends; the scene is unchanged | PASS |
| S05-L3 | `kill -9` of Studio mid-broadcast: relaunch, and the output is desired `Stopped` (no auto-resume). A second `serve` on the same runtime dir exits with `LockHeld`. A duplicate command ID returns the same receipt | PASS |
| S05-L4 | Induced start failure: StartBroadcast to a closed local RTMP port. Record whether a modal dialog blocks the UI (`engine.studio_ui_blocked` within ~15 s after the start; receipt `OutcomeUnknown`/`Failed`). Then `SetLifecycle false` → `true` via `live_engine` recovers to `engine.ready` | outcome recorded; recovery PASS |

Every check also asserts zero customer impact. Cleanup: stop the sink, source, Studio and display. No real platforms are used.

#### 13. Acceptance criteria and definition of done

1. Local tests pass and are nonempty for `state`, `reconciler` and `daemon`, and `tests/architecture.rs` passes.
2. The legacy runtime, including `src/chat/` legacy files, and the deprecated adapters are deleted. `rg 'noalbs::|ChatHandler|UserManager|rust_i18n'` finds nothing in `src/`.
3. One actor, one executor and one supervisor; B1–B3 are enforced by test.
4. S05-L1 to S05-L4 PASS, and all S01–S04 checks still PASS.
5. The measured shutdown time and the S05-L4 outcome are recorded as development hardware.

#### 14. Handoff to later steps

- **Step 06:** `SessionHandle`, the `Principal` constructors, `SessionSnapshot` as JSON, and the `ReloadConfig` and `Shutdown` intents.
- **Step 07:** `EngineInput`, which it extends with `Telemetry`, and snapshot fields to extend.
- **Step 08:** `SceneMode`/`HoldKind`, scene validation, the `RepairMedia` stub, `engine.autoRepair` and `engine.telemetryFreshnessMs`.
- **Step 09:** `PrincipalKind::Chat` and the command model. The legacy chat code is available from Git history (`dab2665`).
- **Step 11:** the `serve` exit codes and the shutdown budget.
- **Steps 06–10:** `tests/architecture.rs`, which each extends.

#### 15. Open questions and assumptions

- 1 Hz Studio polling, per `protocol.md` §9 (D-03).
- No broadcast auto-resume after a daemon restart (old Step 11, D-24, Q-16).
- Single destination unless D-04 is approved.
- The UI-blocked reaction and the IrlosStudio dialog fix (D-25).
- Whether QMessageBox's nested event loop still serves ICP tasks (Q-23; observed in S04-L5/S05-L4).
<!-- STEP 05 END -->

<!-- STEP 06 START -->
### Step 06 — Local admin IPC and the `status` / `command` clients

#### 1. Title and metadata

| | |
|---|---|
| Step | 06 of 13: admin Unix socket (framed JSON-RPC 2.0) served by the daemon; `irlosd status` and `irlosd command` clients; health check |
| Depends on | 05 |
| Dependents | 07–13: every later live check uses it; 10 (the TUI is a client); 11 (container health check); 12 (`seatctl`) |
| Absorbs old steps | 13 (admin socket) |
| Code status at drafting | Not started. The legacy Warp `/ws` API was removed in 01/05 |

#### 2. Objective

Expose `SessionHandle` to local processes through `<runtimeDir>/irlosd.sock`, with:
- the §8.10 framing and limits;
- peer-credential authentication and capability-scoped principals;
- request/response methods;
- coalesced snapshot subscriptions.

Ship `irlosd status [--json|--health]` and `irlosd command <verb>` as the first clients. They become the uniform way later live checks observe and drive the daemon.

#### 3. Why this step exists

Several parties need to inspect and drive a running session:
- operators;
- the TUI;
- container health checks;
- the future provisioning agent (via `seatctl`);
- live tests.

None of them should have to touch IrlosStudio or hold the single ICP connection. A separate local socket, distinct from ICP and never exposed on the network, gives them that access through the state owner. The admin socket therefore cannot become a second control path, and clients that connect, disconnect, stall or misbehave can never affect the broadcast.

#### 4. Starting state

- **Step 05 handoff:**
  - `irlosd serve` running the engine;
  - `SessionHandle` with `submit`, `command_status`, `snapshot` and `subscribe`;
  - private `Principal` constructors;
  - `SessionSnapshot` serializable to JSON;
  - the `Intent`/`OutputTarget`/`ModeRequest` types and the `ReloadConfig`/`Shutdown` intents;
  - `tests/architecture.rs`;
  - `rtmp_sink.sh`, `srt_source.sh`, `tests/live_engine.rs`.
- S01–S05 pass.
- No `src/admin/` exists yet. `cli.rs` already has `Status` and `Command` variants, which return "not implemented".

#### 5. Scope

**In scope:**
- `src/admin/{mod,protocol,server,client}.rs`;
- the `admin` config block;
- the `status` and `command` CLI implementations;
- `irlosd status --health`;
- the admin events;
- `live/lib/admin.sh`.

**Out of scope:**
- The TUI (Step 10).
- Any remote access; there is none in the alpha.
- The provisioning agent. It belongs to IrlosWeb; Step 12 supplies `seatctl`.
- Chat (Step 09).

#### 6. Design specification

**Transport.** An `AF_UNIX` `SOCK_STREAM` socket at `<runtimeDir>/irlosd.sock`.

Startup:
1. Verify the runtime dir is ours, with mode `0700` (D-05).
2. Refuse to start if a live listener answers on the path.
3. Remove a stale socket of ours only while holding `RuntimeLock`.
4. Bind, then `chmod 0660`.

Frames are a u32 big-endian length followed by UTF-8 JSON, 2..65536 bytes; a frame outside those bounds closes the connection. Each request has a 5 s read and write deadline. This is not HTTP, so `curl --unix-socket` does not apply.

**Authentication.** `peer_cred()` on accept maps the peer UID to a principal:

| Peer UID | Principal | Capabilities |
|---|---|---|
| Same as the daemon | `LocalService` | all |
| In `admin.operatorUids` | `LocalOperator{uid}` | all except `Shutdown` and `Config`, unless listed in `admin.operatorCapabilities` |
| In `admin.readOnlyUids` | `LocalReadOnly{uid}` | `Read` |
| Any other | — | logged as `admin.client_rejected`, then closed |

The request body can never claim a UID or a tenant.

**Protocol (JSON-RPC 2.0, `admin/protocol.rs`).** IDs are strings of 1–64 bytes. The first request must be `hello {"protocol_version": 1}`. Its result is `{protocol_version, daemon_version, session_id, epoch, principal, capabilities}`.

| Method | Params | Result | Capability |
|---|---|---|---|
| `status` | — | `SessionSnapshot` JSON | `Read` |
| `submit_command` | `{id, intent, expected_revision?, admission_generation?}` | `CommandReceipt` | per intent (Step 05 rules) |
| `command_status` | `{id}` | `CommandReceipt` | `Read` |
| `failures` | `{cursor?, limit ≤ 50}` | page of sanitized failure records | `Read` |
| `subscribe` | — | `{subscription}`, then notifications `snapshot {revision, snapshot}` coalesced to ≤ 2 Hz | `Read` |
| `unsubscribe` | `{subscription}` | `{}` | `Read` |

`intent` is the Step 05 `Intent`, serialized with `#[serde(tag = "type", rename_all = "snake_case")]`. Examples:

```json
{"type":"start_broadcast","target":"all_enabled"}
{"type":"stop_broadcast","target":{"one":"main"}}
{"type":"set_scene","scene":"Live","ttl_secs":null}
{"type":"set_mode","mode":{"hold":{"kind":"manual","scene":"BRB"}}}
{"type":"set_mode","mode":"auto"}
{"type":"set_recording","active":true}
{"type":"set_lifecycle","running":false}
{"type":"repair_media","source":"ingest"}
{"type":"reload_config"}
{"type":"shutdown"}
```

Errors:
- JSON-RPC codes −32600 to −32603 for protocol faults, and −32601 for an unknown method.
- −32000 with `data: PublicFailure` for domain faults.
- A result over 65536 bytes returns `protocol_blocked`; it is never truncated.
- A parse error closes the connection after the reply.

**Limits.**
- At most 16 clients; the 17th is rejected.
- At most 32 outstanding requests per client; the 33rd gets `busy`.
- At most 128 queued notifications per client. A client that falls behind gets only the latest snapshot, and one that stays more than 5 s behind is disconnected (`admin.client_disconnected reason=slow`).
- Idle connections are allowed.

**Server (`admin/server.rs`).** One accept task, plus one task per connection holding a cloned, principal-scoped `SessionHandle`. Snapshot serialization happens in the connection task, never in the actor. A disconnect cancels only that connection's waits, never accepted intents.

**Client (`admin/client.rs`)**

```rust
impl AdminClient {
    pub async fn connect(path: &Path) -> Result<Self, IrlosError>;                // hello included
    pub async fn status(&mut self) -> Result<SessionSnapshot, IrlosError>;
    pub async fn submit(&mut self, intent: Intent) -> Result<CommandReceipt, IrlosError>;   // random CommandId
    pub async fn wait_receipt(&mut self, id: &CommandId, until: ReceiptPredicate, timeout: Duration) -> Result<CommandReceipt, IrlosError>;
    pub async fn subscribe(&mut self) -> Result<SnapshotStream, IrlosError>;
    pub async fn failures(&mut self, cursor: Option<String>, limit: u16) -> Result<FailurePage, IrlosError>;
}
```

The socket path comes from `--socket`, then `IRLOSD_ADMIN_SOCKET`, then `/run/irlos/irlosd.sock`. If no daemon answers, the client prints "irlosd not running at <path>" and exits 4. **It never starts a daemon.**

**CLI**

```
irlosd command start [--output <id>]      | stop [--output <id>]
               scene <exact name…> [--ttl <s>]  | hold [--admin] [<scene…>] | release [--admin] | auto
               record on|off              | lifecycle running|stopped
               repair <mediaSourceId>     | reset-recovery | reset-repair | reload | shutdown --yes
   common flags: --socket <path>  --wait applied|accepted (default applied)  --timeout <s> (default 30)  --json
irlosd status [--json | --health] [--socket <path>]
```

| Exit code | Meaning |
|---|---|
| 0 | applied, or accepted as requested |
| 5 | rejected (`PublicFailure` printed) |
| 6 | timed out waiting; the receipt is still pending |
| 4 | no daemon |

- `scene` joins the remaining arguments exactly, so multiword names work when quoted.
- `status` prints the engine state, epoch, controller, process, scene and mode, the outputs with their phases, and value ages.
- `--health` exits 0 if the engine is `Ready` or `Degraded` and 1 otherwise; Step 11's health check uses it.

**Config (`admin` block).** Defaults: `operatorUids: []`, `operatorCapabilities: null`, `readOnlyUids: []`, `maxClients: 16`, `maxOutstandingPerClient: 32`, `notificationQueue: 128`. Validation: UIDs must be unique, and limits may not exceed §8.10.

**Events emitted:** `admin.listening`, `admin.client_connected`, `admin.client_rejected`, `admin.client_disconnected` (with a reason).

#### 7. Implementation tasks

**Unit A — protocol and server (checkpoint S06-L1)**
1. Write the `protocol.rs` types with golden JSON tests, including every intent example above.
2. Write `server.rs`: authentication, limits and subscriptions.
3. Wire the admin server into `daemon.rs`. At startup it starts before Studio launches. At shutdown it stops accepting first and closes clients last.

**Unit B — client and CLI (checkpoints S06-L2, S06-L3)**

4. Write `client.rs` and the `status` and `command` implementations.
5. Write `live/lib/admin.sh` with these helpers:
   - `irlosd_status_json`;
   - `irlosd_cmd <verb…>`;
   - `wait_snapshot <python-expr> <timeout>`, which evaluates the expression against the snapshot JSON with stdlib `python3 -I`, so `jq` is not needed.

   Port the S05 live flows to the CLI. All later steps use these helpers.

#### 8. Error handling and recovery

- Errors are `Admin { kind: Frame | Auth | Limit | Protocol | Io }`.
- Client faults affect only that connection.
- `accept` errors such as EMFILE back off from 100 ms to 2 s and are logged at most once per 30 s.
- If the socket file is removed externally, recreate it, holding the lock and only after confirming no other listener exists.
- If the admin server fails to start, `serve` fails before Studio launches.

#### 9. Concurrency and lifecycle

- Snapshots come from `watch`, with latest-value coalescing.
- None of these sends anything to the actor: a client disconnecting, an SSH loss, or a killed `status` process.
- Shutdown: stop accepting, give in-flight replies 500 ms, then close.
- **No admin behaviour can stop a broadcast except an explicit authorized intent.**

#### 10. Security and isolation

- The socket has mode `0660` inside a `0700` runtime dir, and authorization uses peer UID mapping only.
- There is no TCP listener.
- Frames and queues are bounded.
- Snapshots are already redacted (Step 05). Failure pages are sanitized.
- `shutdown` requires `--yes` and the `Shutdown` capability.
- Each session has its own runtime dir, so sessions cannot see each other's sockets; Steps 11–12 enforce the mounts.

#### 11. Local testing

- **Protocol:** golden JSON; `hello` is required; an oversize frame closes the connection; malformed JSON; an unknown method; every intent's JSON shape round-trips.
- **Authorization:** an unauthorized UID is rejected (via an injected credential resolver); a read-only principal cannot submit.
- **Limits:** the 17th client is rejected; the 33rd outstanding request gets `busy`; a slow subscriber is disconnected without affecting the actor (assert actor tick latency); the 2 Hz coalescing bound; size overflow gives `protocol_blocked`.
- **CLI:** exit codes; multiword scene arguments; the `--health` mapping.
- **Architecture:** extend `tests/architecture.rs` so that `admin/` must not import `broadcasting_software` or `supervisor`.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S06-L1 | `irlosd status --json` against a live `serve` with real Studio shows engine `Ready`, scene `BRB` and outputs `Stopped`; `--health` exits 0; `admin.listening` is logged | PASS |
| S06-L2 | `irlosd command scene Live`, then `start`; the sink receives ≥ 20 s; then `stop`, `record on`, `record off`; all reach Applied with exit codes 0 | PASS |
| S06-L3 | Robustness during a live broadcast: 16 subscribers are accepted and a 17th rejected; a stalled reader (`SIGSTOP`ped client) is disconnected; 50 malformed frames are sent; throughout, the sink shows no gap > 2 s | PASS |

Each check also asserts zero customer impact. Cleanup is as in earlier steps. No owner-approved actions are needed.

#### 13. Acceptance criteria and definition of done

1. Local tests pass.
2. `admin/` uses only `SessionHandle`, enforced by `tests/architecture.rs`.
3. No admin path bypasses capability checks.
4. Client faults never change the broadcast (S06-L3).
5. S06-L1 to S06-L3 PASS, and all S01–S05 checks still PASS.
6. `live/lib/admin.sh` exists, and the S05 checks also run through the CLI.

#### 14. Handoff to later steps

- `AdminClient`, the protocol types, and `status`/`command` with their exit codes.
- `live/lib/admin.sh`, for Steps 07–13.
- `--health`, for Step 11.
- `failures` paging and subscriptions, for Step 10.

#### 15. Open questions and assumptions

- The single-UID seat model: `LocalService` is the same UID as Studio (D-05).
- Whether IrlosWeb's agent uses `seatctl`, this socket, or both (Q-17).
- The operator UID scheme on hosts (Q-14).
<!-- STEP 06 END -->

<!-- STEP 07 START -->
### Step 07 — Telemetry ingestion: SRT Live Server and Belabox

#### 1. Title and metadata

| | |
|---|---|
| Step | 07 of 13: normalized telemetry model; single-owner polling runners; SLS and Belabox adapters; facts into the state owner |
| Depends on | 05 (actor, `EngineInput`), 06 (live observation) |
| Dependents | 08 (policy consumes the outcomes), 09 (`!bitrate`), 10 (gauges), 13 |
| Absorbs old steps | 7 (SLS telemetry), 8 (Belabox adapter) |
| Code status at drafting | Partial (legacy). `src/stream_servers/sls.rs` (178 lines) reads camelCase `publishers[publisher]`; unreachable maps to Offline and 0 to Previous. `belabox.rs` (144 lines) reads `{bitrate, rtt, dropped_pkts}`; 0 maps to Offline and 1 to Previous. Both re-fetch on every call. The other providers were removed in 01 |

#### 2. Objective

Replace the legacy pull-on-demand providers with one polling runner per configured provider. Each runner produces truthful, normalized, sequence-numbered `PollOutcome`s with explicit sample quality and freshness evidence, and publishes them into the state owner as `EngineInput::Telemetry`.

Two schemas are supported: the source-supported `noalbs_sls_json` and `noalbs_belabox_json`. Other schemas are refused until sanitized samples exist.

#### 3. Why this step exists

Scene automation is only as good as its measurements. The legacy code has three faults:
- it turns an unreachable stats page into "Offline", which switches viewers to BRB while the feed may be fine;
- it treats zero as "keep previous";
- it re-polls for every reader.

The owner chose the SRT server's stats as the source. They also check Belabox through belabox.net, and RECON found one tenant pulling from Belabox cloud, whom the relay's stats cannot see. Both adapters are therefore needed, each bound to the media route it actually measures.

#### 4. Starting state

- **Steps 05–06 handoff:**
  - the actor with `EngineInput`;
  - `SessionSnapshot` (extendable);
  - `engine.telemetryFreshnessMs`;
  - `irlosd status --json` and `live/lib/admin.sh`;
  - S01–S06 passing.
- The legacy `sls.rs`, `belabox.rs` and `stream_servers/mod.rs` (typetag registry) are still present, used only by config parsing.
- Read the legacy `sls.rs` `Stat` (`#[serde(rename_all = "camelCase")]`: `bitrate`, `bytesRcvDrop`, `bytesRcvLoss`, `mbpsBandwidth`, `mbpsRecvRate`, `msRcvBuf`, `pktRcvDrop`, `pktRcvLoss`, `rtt`, `uptime`).
- RECON records the following facts from the relay's source:
  - the only route is `/stats`;
  - `?publisher=<key>` returns a single publisher;
  - **`?reset` zeroes the interval counters for every reader**;
  - `pktRcvLoss`, `pktRcvDrop`, `bytesRcvLoss`, `bytesRcvDrop` and `mbpsRecvRate` are interval values;
  - `rtt`, `msRcvBuf` and `mbpsBandwidth` are instantaneous;
  - `bitrate` is in kbps and `uptime` in seconds.

#### 5. Scope

**In scope:**
- `stream_servers/telemetry.rs` (types and runner);
- the rewrite of `sls.rs` and `belabox.rs` as `TelemetryProvider`s, and removal of the typetag registry;
- the `telemetry` config block and its migration from `switcher.streamServers[]`;
- `EngineInput::Telemetry` and the snapshot telemetry fields;
- `live/fakes/fake_stats.py`;
- the optional `ModemHealth` type (its adapter stays disabled without fixtures).

**Out of scope:**
- Severity classification, hysteresis, stale-data BRB and provider selection for switching (Step 08).
- Chat display (Step 09).
- XML or other SLS schemas, and `belabox_interfaces_v1`. These are enabled only after D-13/Q-06/Q-07 samples arrive.
- Relay-side authorization or filtering proxies (deployment; D-13).

#### 6. Design specification

**Types (`stream_servers/telemetry.rs`)**

```rust
pub enum SampleQuality { Valid, PublisherAbsent, SentinelHold, Unavailable, Disabled }
pub enum FreshnessEvidence { ResponseOnly, ValidatedSensorTime, CurrentOnRequestContract }
pub enum CounterKind { Cumulative, Interval, Unknown }
pub struct StreamHealth { pub bitrate_kbps: Option<u64>, pub rtt_ms: Option<f64>, pub recv_buffer_ms: Option<u64>,
    pub bandwidth_mbps: Option<f64>, pub uptime_secs: Option<u64>, pub pkt_drop: Option<u64>, pub pkt_loss: Option<u64>,
    pub counter_kind: CounterKind, pub pkt_drop_per_s: Option<f64>, pub pkt_loss_per_s: Option<f64>,
    pub modems: Option<Vec<ModemHealth>> }
pub struct ModemHealth { pub interface_id: String /* ≤128 B, sanitized */, pub carrier: Option<String>, pub bitrate_kbps: Option<u64>,
    pub rtt_ms: Option<f64>, pub loss_percentage: Option<f64>, pub state: Option<ModemState> }      // at most 32 entries
pub struct PollOutcome { pub provider: ProviderId, pub route_id: RouteId, pub provider_generation: Generation, pub sequence: u64,
    pub polled_at: Instant, pub quality: SampleQuality, pub freshness: FreshnessEvidence,
    pub sample: Option<StreamHealth>, pub failure: Option<TelemetryFailureKind> }
#[async_trait] pub trait TelemetryProvider: Send { async fn poll_once(&mut self) -> PollOutcome; }
pub struct TelemetryRunner;   // spawn(provider, interval, tx: mpsc::Sender<EngineInput>) -> JoinHandle — one task per provider
```

**Runner**
- **Timing.** Polls every `pollIntervalMs` (default 1000) with `MissedTickBehavior::Skip` and never overlapping. `timeoutMs` (default 750) covers connect, headers and body end to end.
- **Delivery.** Outcomes go to the actor with `try_send`. If the queue is full, only the newest is kept and drops are counted.
- **Reads.** Every reader (chat, status, TUI) sees only the actor's snapshot. No read path triggers HTTP.
- **Rates.** Per-second rates are computed only for `CounterKind::Cumulative`, from consecutive valid samples. The baseline resets on the first sample, an uptime decrease, a counter decrease, a generation change or elapsed ≤ 0.
- **Lifecycle.** A runner panic or exit is a critical-task failure (Step 05 rules). Runners are cancelled at shutdown.

**Adapters**

| Schema | Record | Quality rules |
|---|---|---|
| `noalbs_sls_json` | `publishers[<publisher>]` with the camelCase fields above | A valid root without the exact key is `PublisherAbsent`. A malformed root, record or required `bitrate` is `Unavailable`. `bitrate == 0` is `SentinelHold` by default; `zeroBitrateMeaning: "offline"` makes it `Valid` with 0 and needs endpoint evidence. Negative or non-finite values are invalid; an invalid required field gives `Unavailable` and an invalid optional field is dropped. Units: `bitrate` kbps; `rtt` and `msRcvBuf` ms. Drop/loss counters default to `counterKind: "unknown"`: they are interval values whose meaning depends on other readers, so they are shown raw and get no rates |
| `noalbs_belabox_json` | `publishers[<publisher>]` with `{bitrate: i64, rtt: f64, dropped_pkts: i32}` | `0` is `Valid` with 0 (Step 08 treats it as measured offline). `1` is `SentinelHold`. Negative values are `Unavailable`. RTT is optional. `dropped_pkts` uses `counterKind` from config (default `unknown`) |

- **The SLS adapter never sends `?reset`.** It may send `?publisher=<key>` only when `publisherQuery: true` *and* a sanitized fixture of that response shape exists (Q-06). Otherwise it fetches `/stats` and discards other publishers immediately.
- **Freshness.**
  - An HTTP receipt alone is `ResponseOnly`.
  - `CurrentOnRequestContract` requires `freshnessContract: "current_on_request"` plus a non-empty `freshnessJustification` string. The justification records the sls version and the source evidence that `/stats` computes values from live socket statistics at request time (RECON cites `CSLSManager::create_json_stats_for_publisher`). Supplying that evidence is an owner/provisioning task (D-13, Q-06).
  - `ValidatedSensorTime` needs a timestamped schema and is not implemented for these adapters.
- **Route binding.** `routeId` names the media route the provider measures, and it must match `studio.mediaSources[].routeId`.
  - For a relay tenant, the SLS provider is priority 0 on the relay route.
  - For a Belabox-cloud tenant, whose `inputUrlSecret` points at Belabox cloud, the Belabox provider is priority 0 on that route.
  - A provider never measures a route it does not serve, and priority never changes video.

**HTTP**
- One `reqwest::Client` per provider, built after validation, with the timeout, `redirect(Policy::none())` and verified TLS.
- Plaintext `http://` is allowed only for loopback, or with `allowPlaintext: true` for a provisioned private address. It is never allowed with credentials unless the address is loopback.
- The body is capped at 1 MiB, enforced while streaming and decompressing, and JSON depth is bounded.
- `authMode`:

  | Value | Request |
  |---|---|
  | `none` | no auth header |
  | `raw_header` | `Authorization: <secret>` (the legacy `apiKey` behaviour) |
  | `bearer` | `Authorization: Bearer <secret>` |

**Config (`telemetry` block)**

```json
"telemetry": { "providers": [
  { "id": "relay", "type": "srt_live_server", "enabled": true, "routeId": "main", "priority": 0,
    "statsUrl": "<sensitive URL>", "publisher": "<sensitive publisher key>", "schema": "noalbs_sls_json",
    "publisherQuery": false, "authMode": "none", "authSecret": null, "pollIntervalMs": 1000, "timeoutMs": 750,
    "zeroBitrateMeaning": "hold", "counterKind": "unknown", "freshnessContract": null, "freshnessJustification": null, "allowPlaintext": false,
    "overrideScenes": null, "dependsOn": null, "backupScenes": null } ] }
```

- For Belabox: `"type": "belabox"`, `"schema": "noalbs_belabox_json"`, `"counterKind": "unknown"`. Its stats URL and publisher are the per-key Belabox cloud values, both secret-bearing.
- `statsUrl` and `publisher` are `Sensitive<String>`.
- Validation:
  - at most 16 providers with unique IDs;
  - `routeId` exists;
  - interval 250–60000 ms and timeout below the interval;
  - an unknown `schema` fails as unsupported;
  - a `freshnessContract` without its justification field is rejected.
- `overrideScenes`, `dependsOn` and `backupScenes` are carried for Step 08 to validate.
- **Migration.** Legacy `switcher.streamServers[]` entries (`SrtLiveServer`, `Belabox`) map here, with `name` becoming `id` and legacy `apiKey` becoming `raw_header`. `config/legacy.rs` parses the legacy `streamServer` objects from `serde_json::Value` (type tag plus fields), so the typetag registry can be removed. Priority sorts ascending with missing priorities **last**. This corrects the legacy behaviour, where `Option` sorting put `None` first (`config.rs:79`).

**Snapshot extension.** Per provider: `id`, `routeId`, `quality`, `freshness`, sample values, `counterKind`, `last_attempt_age_ms`, `last_valid_age_ms`, `last_measurement_age_ms`, `failure` (category only) and `drops_counted`.

**Events emitted:** `telemetry.provider_started`, `telemetry.quality_changed` (from/to), `telemetry.poll_failed` (throttled to one per 30 s per identical failure, with a count).

#### 7. Implementation tasks

**Unit A — model and adapters (local)**
1. Write `telemetry.rs`, the SLS and Belabox parsers as pure functions `parse(&[u8], &AdapterConfig) -> (SampleQuality, Option<StreamHealth>)`, and the rate/baseline logic. Fixtures go in `tests/fixtures/telemetry/`, with synthetic publisher keys only.

**Unit B — runner, HTTP, wiring (checkpoints S07-L1, S07-L2, S07-L3)**

2. Write the runner and HTTP client, the config block and its migration, and `EngineInput::Telemetry`. The actor stores outcomes in the snapshot; no policy acts on them yet. Remove the legacy typetag registry.
3. Write `live/fakes/fake_stats.py` (stdlib):
   - It serves both schemas on `127.0.0.1:<port>`.
   - `POST /script` with a JSON curve such as `[{"t":0,"bitrate":5000,"rtt":80},…]` sets the values.
   - Fault modes: `timeout`, `500`, `oversize`, `malformed`, `absent`, `zero`, `one`, `foreign` (adds unrelated publishers).
   - It logs every query string it receives, so `?reset` can be detected.
   - It runs under `start_tracked`.

#### 8. Error handling and recovery

- Failures are outcomes (`Unavailable` plus `TelemetryFailureKind`), never errors that stop the runner.
- Identical warnings are throttled (`telemetry.poll_failed`).
- `HttpAuth` stays visible as a configuration problem and does not speed up retries.
- There is no backoff beyond the interval: polling is cheap and bounded.
- A failed poll keeps the last displayed values, but Step 08 resets its evidence on it.

#### 9. Concurrency and lifecycle

- One task per provider, independent of Studio. Polling continues while Studio restarts.
- The channel is bounded, and the actor never waits on telemetry.
- **Telemetry cannot stop or start an output, change a scene or launch Studio.** In this step it changes nothing but the snapshot.

#### 10. Security and isolation

- URLs, publisher keys and auth secrets are never logged, snapshotted or returned; snapshots show only the provider ID.
- Redirects are disabled.
- There is no TLS bypass.
- Bodies are bounded.
- Foreign records are discarded at once.
- `?reset` is never sent, so other readers' figures are not corrupted.
- Daemon-side filtering does not make a shared stats endpoint tenant-safe. Step 12 records the need for a seat-scoped endpoint or a filtering proxy (D-13).

#### 11. Local testing

- **Parser cases:**
  - healthy, low;
  - zero under both `zeroBitrateMeaning` values; Belabox `1`;
  - absent publisher with a valid root; malformed root;
  - missing RTT; negative and NaN values; overflow; numeric strings rejected;
  - foreign publishers discarded.
- **Rates:** no rates for `unknown`/`interval` counters; for `cumulative`, the first sample, counter decrease, uptime decrease and generation change.
- **Runner** (paused time plus a loopback mock HTTP server):
  - no overlap on a slow response;
  - the timeout is honoured;
  - oversize and redirect responses are rejected;
  - cached reads make no extra HTTP requests (count them);
  - **no request ever contains `reset`**.
- **Redaction:** a canary publisher and URL are absent from snapshots and logs.
- **Migration:** a legacy `streamServers` fixture; missing priority sorts last.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S07-L1 | `serve` with an SLS provider pointing at `fake_stats.py`, curve 5000 → 800 → 0 → absent → 4000 kbps: `wait_snapshot` shows quality `Valid → Valid → SentinelHold → PublisherAbsent → Valid`, each within 2 s of the change. The fake's query log shows no `reset` | PASS |
| S07-L2 | Faults `timeout`, `500`, `oversize` and `malformed` each give `Unavailable` with the right category, and the runner keeps its interval. A broadcast to `rtmp_sink.sh` running throughout is unaffected: no output state change, and the sink has no gap > 2 s | PASS |
| S07-L3 | A Belabox provider on a second route (configured as a Belabox-cloud-fed media source pointing at the SRT test source) sees 0, 1 and a positive value, giving `Valid(0)`, `SentinelHold` and `Valid`. The `foreign` publishers never appear in the snapshot or logs (grep for the fake foreign key) | PASS |

Optional with OA-15: a real `srt-live-server` built in the dev root and fed by `srt_source.sh`. Compare its `/stats` JSON with the `noalbs_sls_json` fixture (D-13). Cleanup: stop the fakes, Studio and the sink.

#### 13. Acceptance criteria and definition of done

1. Local tests pass.
2. No read path issues HTTP, and no request contains `reset` (both proved by tests and S07-L1).
3. A network failure never produces a zero or an Offline value.
4. Telemetry has no control effect (tests and S07-L2).
5. No secret or foreign publisher leaks (canaries and S07-L3).
6. S07-L1 to S07-L3 PASS, and all S01–S06 checks still PASS.

#### 14. Handoff to later steps

- **Step 08:** ordered `PollOutcome`s in the actor, with provider, route, generation, sequence and freshness, plus provider priorities and the scene override/dependency settings.
- **Steps 09–10:** the snapshot telemetry fields.
- **Step 13:** `fake_stats.py` curves and faults.

#### 15. Open questions and assumptions

- The deployed relay's format, version, auth and freshness evidence, and whether its endpoint is seat-scoped (D-13, Q-06).
- Belabox usage per tenant, and the per-modem fields (Q-07).
- That the `?publisher=` response shape matches the root map (Q-06).
- Zero means `hold` by default (old Step 7 decision, preserved).
- Production stats endpoints are provisioning data and are never written into this repository.
<!-- STEP 07 END -->

<!-- STEP 08 START -->
### Step 08 — Scene policy: hysteresis, holds, stale-data protection and media repair

#### 1. Title and metadata

| | |
|---|---|
| Step | 08 of 13: pure `HysteresisPolicy` (3 bad / 5 good), provider selection, Auto arbitration against holds, stale-data BRB, guarded media repair |
| Depends on | 05 (actor, `SceneMode`), 06 (live driving), 07 (`PollOutcome`) |
| Dependents | 09 (`!brb`, `!live`, `!auto`, `!fix`), 10, 13 |
| Absorbs old steps | 9 (hysteresis switcher); 11 (policy parts: stale-data BRB, media repair, Auto rules) |
| Code status at drafting | Partial (legacy, deleted in 05). It used one symmetric `retry_attempts` (default 5), instant-recover flags, an offline-timeout stream stop and direct `switch_scene` calls whose errors were swallowed |

#### 2. Objective

Automate scene choice without ever stopping an output:
- A pure, deterministic `HysteresisPolicy` turns ordered telemetry outcomes into scene proposals.
- The actor arbitrates those proposals against human holds.
- When every measurement is stale or unavailable, it cuts to a validated safe BRB.
- It proposes a media repair only when ingest is healthy but Studio's media source has faulted. Repairs run through the controller under strict budgets.

#### 3. Why this step exists

This is the operator's core job: LIVE when the feed is good, LOW when it degrades, BRB when it is gone, and recovery only once the feed is truly back. Old Step 9 specified a complete transition matrix and sentinel rules, and old Step 11 specified arbitration and repair guards. Built after the state owner and telemetry, these become a policy object *inside* the single writer, with no second executor.

#### 4. Starting state

- **Steps 05–07 handoff:**
  - The actor receives `EngineInput::Telemetry(PollOutcome)` with sequence, generation, freshness and route.
  - `SceneMode`/`HoldKind`, `SetScene`/`SetMode` arbitration and TTL semantics.
  - Scene validation against the current list.
  - The `RepairMedia` intent, which still returns `unsupported`.
  - `engine.autoRepair` and `engine.telemetryFreshnessMs`; `studio.templateScenes`.
  - `Obsv5` maps `RepairMedia`. `UnixSocketClient` has no repair capability unless the Step 03 Unit D companion shipped.
  - `fake_stats.py`.
  - S01–S07 passing.

#### 5. Scope

**In scope:**
- `switcher.rs` (new contents: the pure policy);
- `reconciler/policy.rs` additions: `StaleDataGuard`, `MediaRepairPolicy`;
- the `switcher` config block and its legacy migration rows;
- the snapshot policy fields;
- the policy and media events.

**Out of scope:**
- Chat aliases (Step 09).
- TUI display (Step 10).
- **Encoder bitrate changes, never done.** LOW changes composition only (D-11).
- Studio-side frame-age instrumentation (D-04).
- The legacy offline-timeout stop, raid stop, transcoding checks and starting-scene automation. These are dropped (D-17).

#### 6. Design specification

**Policy (`switcher.rs`)**

```rust
pub enum Severity { Unknown, Normal, Low, Offline }                 // order Normal < Low < Offline
pub struct PolicyInput<'a> { pub outcome: &'a PollOutcome, pub now: Instant, pub config_generation: Generation,
    pub policy_generation: Generation, pub automation_allowed: bool }
pub enum PolicyDecision { Hold { reason: HoldReason }, Propose(SceneProposal) }
pub struct SceneProposal { pub target: String, pub severity: Severity, pub reason: &'static str, pub provider: ProviderId,
    pub sample_sequence: u64, pub config_generation: Generation, pub policy_generation: Generation }
impl HysteresisPolicy { pub fn evaluate(&mut self, input: PolicyInput) -> PolicyDecision;
                        pub fn reset(&mut self, why: ResetReason); pub fn registers(&self) -> PolicyRegisters }
```

**Classification of a fresh outcome.** Evaluate the rules in order; the first match wins:

| # | Condition | Result |
|---|---|---|
| 1 | `PublisherAbsent`, or a schema-confirmed offline zero (Belabox `0`, or SLS `0` with `zeroBitrateMeaning: "offline"`) | `Offline`, immediately |
| 2 | Known bitrate ≤ `offlineKbps`, or known RTT ≥ `offlineRttMs` | `Offline` |
| 3 | Known bitrate ≤ `lowKbps`, or known RTT ≥ `lowRttMs` | `Low` |
| 4 | All enabled threshold metrics known **and** freshness is better than `ResponseOnly` | `Normal` |
| 5 | Otherwise | `Unknown` (hold) |

`SentinelHold`, `Unavailable` and `Disabled` are never votes.

**Transitions**
- **Degrading** needs `degradeSamples` (3) consecutive, distinct, matching samples. Rule 1 applies immediately from Normal or Low.
- **Recovering** needs `recoverSamples` (5) consecutive matching samples whose freshness is better than `ResponseOnly`; this includes Offline → Low.
- **Startup** severity is `Unknown`. The session stays on the safe BRB until 5 such samples arrive.
- **Evidence resets** on any of:
  - an unknown or failed sample, or a sequence gap;
  - a change of provider, provider generation or route;
  - a config or policy change;
  - entering or leaving a manual mode.
- Repeated sequences and ticks never vote, and counters saturate.

> **Consequence the owner must decide (D-13).** With `ResponseOnly` freshness, the policy can cut *to* BRB/LOW but can **never** recover automatically, not even from the startup BRB. Automatic LIVE therefore needs a `freshnessContract` backed by evidence (Step 07). The alternative is for the owner to relax this rule, which was old Step 7's decision and is preserved here. Live checks use a fake contract and record that they do.

**Provider selection (actor)**
- **Candidates** are the enabled providers whose `routeId` equals the active media source's route, ordered by ascending priority (missing priorities last, ties broken by declaration order).
- **Selection** picks the first fresh, usable candidate.
  - If its publisher is `PublisherAbsent`, the next candidate may be used.
  - If every candidate's publisher is absent, the result is `Offline`; if every candidate is unavailable, it is `Unknown`.
  - `SentinelHold` holds; it never skips ahead.
- **`dependsOn`:** if the dependency is confirmed Offline, the selected provider's `backupScenes` apply; if it is Unknown or SentinelHold, proposals are blocked.
- **Targets** come from `overrideScenes`, or else from global `switcher.scenes`.
- The dependency graph must be acyclic and reference known IDs; this is validated at load.
- Changing provider resets evidence. Telemetry never changes the video URL.

**Arbitration (actor, `reconciler/policy.rs`)**
- **Gate.** A proposal applies only when all of these hold: mode `Auto`, `switcher.enabled`, the controller is Ready, and, if `onlySwitchWhenStreaming` is set, at least one output is confirmed Active. Otherwise it is recorded in the snapshot and dropped.
- **Priority.** An applied proposal becomes an internal scene intent with automation priority, below all human intents. It is deduplicated against the pending and applied targets, then dispatched as one `SetScene`. `policy.proposal` records severity, count, provider and reason.
- **Overrides and holds.** When a `ManualOverride` expires, the mode returns to `Auto` and evidence resets. Policy, repair and telemetry never clear holds.

**`StaleDataGuard`.**
- **Trigger:** in `Auto`, every eligible provider's `last_measurement_age_ms` exceeds `engine.telemetryFreshnessMs` (3000), or every provider is Unavailable.
- **Action:** request `templateScenes.brb` once per stale episode (`policy.stale_data_brb`). Recovery follows the normal rules.
- **Limits:** it never fires in `ManualOverride` or `Hold`, and it never touches outputs or recording.

**`MediaRepairPolicy`**
- **Automatic proposals** (`engine.autoRepair`) need both:
  - 5 consecutive Normal samples on the media source's route;
  - the media source observed `error`, `ended` or `stopped` while playback is expected, or an instrumented frame age above 5 s (only with D-04).
- **Never evidence:** paused or intentionally inactive sources, `PLAYING`, or a non-advancing cursor.
- **Explicit `RepairMedia`** with `MediaRepair` capability may run while ingest is Unknown. Without frame evidence it ends `CompletedUnverified`.
- **Budgets:**
  - one repair in flight per source;
  - acknowledgement within the controller request timeout, observation within 10 s;
  - cooldown 30 s, then 60 s, then 120 s;
  - at most 3 attempts per source per process per 5 min, then latched until `ResetRepairBudget` or a new process generation.
- **Uncertainty.** A timed-out repair is `OutcomeUnknown` and is not replayed.
- **In Auto**, a confirmed media fault selects the safe BRB while the repair proceeds.
- **A repair never changes** scene intent, holds, outputs or URLs, and never restarts Studio.
- **Without frame instrumentation**, two fresh `playing` observations close the fault episode with `CompletedUnverified`.
- **Unsupported controllers.** A controller without `media_repair` for the source gets `unsupported` before dispatch.

**Config (`switcher` block)**

```json
"switcher": { "enabled": true, "onlySwitchWhenStreaming": true, "degradeSamples": 3, "recoverSamples": 5,
  "manualOverrideSeconds": 60, "scenes": { "normal": "Live", "low": "Low", "offline": "BRB" },
  "thresholds": { "lowKbps": 1200, "offlineKbps": 300, "lowRttMs": 1500, "offlineRttMs": null } }
```

Validation:
- counts 1–100 and TTL 1–86400 s;
- `offlineKbps` < `lowKbps`;
- `lowRttMs` < `offlineRttMs` when both are set;
- every scene must be a `studio.templateScenes` value or be declared in the template.

Migration (rows owned here):

| Legacy key | Canonical |
|---|---|
| `bitrateSwitcherEnabled` | `enabled` |
| `onlySwitchWhenStreaming` | `onlySwitchWhenStreaming` |
| `retryAttempts` | `degradeSamples`, only when no new keys exist (`recoverSamples` stays 5) |
| `triggers.low` / `offline` | `lowKbps` / `offlineKbps` |
| `triggers.rtt` / `rttOffline` | `lowRttMs` / `offlineRttMs` |
| `switchingScenes` | `scenes` |
| `instantlySwitchOnRecover`, `autoSwitchNotification`, `optionalOptions.*` | dropped, with a warning (D-17) |

Existing provisioned thresholds are kept as they are. RECON shows a tenant with low 450 kbps, offline 400 kbps and RTT 1500 ms. The example values above are for new configs only.

**Snapshot extension:** mode; severity; candidate and count; selected provider; last proposal and reason; stale-episode flag; per-source media phase and repair state, attempts and cooldown.

**Events emitted:** `policy.proposal`, `policy.stale_data_brb`, `media.repair_proposed`, `media.repair_dispatched`, `media.repair_result`.

#### 7. Implementation tasks

**Unit A — pure policy (local)**
1. Write the pure policy in `switcher.rs`, with table-driven tests over the full matrix.
2. Add the config block and its migration rows.

**Unit B — actor integration (checkpoints S08-L1, S08-L2)**

3. Wire provider selection, arbitration and `StaleDataGuard` into the actor.
4. Extend `fake_stats.py` curves for the checks. Live configs use a fake `freshnessContract` with a test justification, and the run log records that.

**Unit C — media repair (checkpoint S08-L3)**

5. Add `MediaRepairPolicy` and enable the `RepairMedia` intent.
6. Add media observations to the executor sweep, for provisioned sources only, and add the snapshot fields.

#### 8. Error handling and recovery

- A missing target scene blocks that target (`invalid_scene`) and is reported once. There is no fuzzy fallback and no loop.
- A controller failure during an automatic switch is retried under the Step 05 budgets; a newer proposal or human intent supersedes it.
- Repair failures follow the repair budgets. Exhaustion reports `recovery_exhausted`.
- Nothing in this step stops an output or Studio.

#### 9. Concurrency and lifecycle

- The policy runs synchronously inside the actor, with no tasks or I/O.
- Repair and scene commands go through the executor like any other command.
- A process relaunch resets policy evidence and repair work (new generation) but keeps the original `ManualOverride` deadline.
- **Nothing here can stop an output:** BRB is only a change of composition.

#### 10. Security and isolation

- Proposals contain only configured scene names, and snapshot reasons are static strings.
- Only bitrate and RTT numbers are logged.
- Policy state is per session. The two-engine isolation test is extended to cover it.

#### 11. Local testing

- **Transitions:**
  - all six severity transitions, plus `Unknown` at startup;
  - exact threshold equality;
  - zero and absence under both `zeroBitrateMeaning` values; Belabox 0 and 1;
  - **`ResponseOnly` never recovers**;
  - interrupted candidates, gaps and repeated sequences.
- **Providers:** priority with missing-last ordering; absence fall-through; route filtering (a Belabox provider on another route is ignored); dependencies and backup scenes; a dependency cycle is rejected.
- **Arbitration:**
  - Auto gating by `onlySwitchWhenStreaming`;
  - override expiry needs new evidence;
  - holds survive repair and stale data;
  - the stale guard fires once per episode.
- **Repair:** the evidence rules; the cooldown sequence; the budget latch; `OutcomeUnknown` is never replayed; a new generation invalidates repair work; `unsupported` over ICP v1.0.
- **Forbidden actions:** assert that no scenario ever stops an output or changes lifecycle.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S08-L1 | Broadcast to `rtmp_sink.sh` in Auto. After the 5-sample warm-up from the startup BRB to `Live`, run the curve 5000 → 200 kbps for 5 s → 4000 kbps. The 3rd low sample switches to `BRB` (offline threshold) and the 5th fresh good sample returns to `Live`. The sink stays continuous, and timings are recorded as development hardware | scene sequence `Live → BRB → Live`; 0 output state changes |
| S08-L2 | `command hold BRB`, then low and high curves: no switch. `command auto`, then a fresh 5-sample recovery to `Live`. A 4 s fake stats `timeout` triggers stale-data BRB once. With `freshnessContract` removed, recovery never happens | PASS |
| S08-L3 | Media repair with `software.type=obs_v5`, **inside the dev box or a private network namespace**. Stop `srt_source.sh` while the stats fake reports healthy, then restart it: media fault → BRB → repair dispatched → `CompletedUnverified` or recovery. Three forced faults latch the budget. Over ICP, `irlosd command repair ingest` returns `unsupported` | PASS |

Each check asserts zero customer impact. Cleanup is as in earlier steps. No owner-approved actions are needed.

#### 13. Acceptance criteria and definition of done

1. The policy is pure, with zero controller or I/O calls in its tests, and local tests pass.
2. Automation never stops outputs, never overrides holds, and never recovers on `ResponseOnly` freshness.
3. Repair obeys all of its budgets.
4. S08-L1 to S08-L3 PASS, and all S01–S07 checks still PASS.

#### 14. Handoff to later steps

- **Step 09:** `!brb` and `!live` map to sticky `SetScene` (ttl 0), `!auto` to `SetMode Auto`, `!scene` to a timed `SetScene`, and `!fix` to `RepairMedia`, each with its Step 08 receipt behaviour.
- **Step 10:** shows the policy registers and repair state.
- **Step 13:** reuses the scenario curves.

#### 15. Open questions and assumptions

- The relay's freshness evidence decides whether automatic recovery can work in production at all (D-13, Q-06).
- Scene naming (D-14).
- Frame-age instrumentation and repair over ICP need the companion (D-04).
- Production tenants keep their migrated thresholds (Q-20).
<!-- STEP 08 END -->

<!-- STEP 09 START -->
### Step 09 — Chat command gateway: Twitch and Kick

#### 1. Title and metadata

| | |
|---|---|
| Step | 09 of 13: platform-neutral chat gateway. Twitch uses TLS IRC; Kick uses signed webhooks inbound and its HTTP API outbound. Commands become intents, with permissions, cooldowns, rate limits, a replay ledger and replies |
| Depends on | 05 (`SessionHandle`, `PrincipalKind::Chat`), 06 (live driving), 07 (cached telemetry for `!bitrate`), 08 (scene, mode and repair semantics) |
| Dependents | 10 (chat screen), 11 (webhook socket mount), 12, 13 |
| Absorbs old steps | 12 (chat gateway) |
| Code status at drafting | Legacy `src/chat/` and `twitch_pubsub.rs` were **deleted in Step 05** and remain in Git history at `dab2665`. The legacy Twitch code works but lowercases the bot token (`twitch.rs:32-33`). It grants admin by *username* (`chat_handler.rs:299`) and folds badges in an order-dependent way. Kick was an anonymous Pusher feed whose `send_message` only logged (`kick.rs:72-74`) |

#### 2. Objective

Let the streamer and approved moderators control their own session from Twitch or Kick chat. Every command is authenticated by stable provider identities and authorized per command. It is rate-limited and deduplicated, then submitted as a typed intent through `SessionHandle`. The sender gets a short reply. If chat is unavailable, misconfigured or flooded, the broadcast continues untouched.

#### 3. Why this step exists

Chat is how a streamer in the field controls IRLOS (`!start`, `!brb`). The legacy code gets trust wrong:
- usernames act as authority;
- tokens are mangled;
- Kick messages cannot be attributed to a sender;
- commands bypass any state owner.

The documented contract (old Step 12, plus the architecture report §4.6) replaces this with:
- provider-authenticated identity;
- per-seat capabilities;
- signed Kick events;
- a durable anti-replay ledger.

#### 4. Starting state

- **Steps 05–08 handoff:**
  - `SessionHandle` scoped per principal; the command model and receipts;
  - `SetScene` TTL semantics (`ttl_secs: Some(0)` means a sticky hold);
  - `SetMode::Auto` and `RepairMedia`;
  - telemetry in the snapshot;
  - `live/lib/admin.sh`;
  - S01–S08 passing.
- No chat code exists in the tree.
- `twitch-irc` was removed in 05 and is re-added here (6.1).
- No HTTP server dependency remains after warp's removal.
- Read the legacy code for reuse with `git --no-pager show dab2665:src/chat/twitch.rs` (and `kick.rs`, `chat_handler.rs`, `mod.rs`).

#### 5. Scope

**In scope:**
- `src/chat/{mod,gateway,commands,permissions,ledger,twitch,kick}.rs`;
- the `chat` config block and its legacy migration rows;
- the webhook Unix HTTP endpoint at `<runtimeDir>/webhook/chat-webhook.sock`;
- token validation and refresh;
- the replay ledger;
- `live/fakes/fake_twitch_irc.py` and `live/fakes/fake_kick.py`;
- the manual platform checklist entries.

**Out of scope:**
- The TLS edge proxy that receives Kick webhooks from the internet. This is deployment infrastructure: Step 12 documents the mount, and D-10 covers approval.
- OAuth onboarding UI (IrlosWeb).
- Raid and host auto-stop, which is dropped (D-17).
- Localization; replies are English-only (D-12).
- Any chat-triggered config, URL, lifecycle or shutdown change (never allowed).

#### 6. Design specification

**Gateway (`chat/gateway.rs`)**

```rust
pub struct AuthenticatedChatMessage { pub provider: ChatProvider /* Twitch | Kick */, pub broadcaster_id: String,
    pub sender_id: String, pub sender_label: Sensitive<String>, pub roles: RoleEvidence /* broadcaster, moderator */,
    pub message_id: String, pub sent_at: SystemTime, pub text: String /* ≤ 512 bytes */ }
#[async_trait] pub trait ChatAdapter: Send {
    async fn next(&mut self) -> Result<AuthenticatedChatMessage, IrlosError>;
    async fn reply(&self, to: &AuthenticatedChatMessage, text: &str) -> Result<(), IrlosError>;
    fn state(&self) -> AdapterState; }
```

An adapter emits a message only after transport-level authentication: TLS IRC with tags, or a verified webhook signature. The gateway then:
1. Rejects messages whose `broadcaster_id` is not the configured channel. This includes shared-chat messages from other rooms.
2. Checks age against the provider timestamp: older than 120 s, or more than 30 s in the future, is rejected.
3. Parses the command.
4. Resolves permissions.
5. Applies cooldowns and rate limits.
6. Records the ledger entry atomically.
7. Builds a `CommandEnvelope` whose `CommandId` is the hex SHA-256 of `(provider, broadcaster_id, message_id)`, truncated to 64 characters.
8. Submits it through a `SessionHandle` scoped to `PrincipalKind::Chat{provider, sender_id}` with the resolved capabilities.
9. Replies with the receipt outcome.

**Commands (`chat/commands.rs`).** The prefix is configurable (default `!`), and so are the aliases. The defaults are the documented set plus `auto`:

| Command (aliases) | Intent | Capability | Reply (≤ 200 chars) |
|---|---|---|---|
| `start` (`go`) | `StartBroadcast{AllEnabled}` (or `start <outputId>`, if permitted) | `Broadcast` | "Starting…", then "Live." or "Start failed: <code>" |
| `stop` (`end`) | `StopBroadcast{AllEnabled}` (or one output) | `Broadcast` | "Stopping…", then "Stopped." |
| `brb` | `SetScene{templateScenes.brb, ttl_secs: Some(0)}` (sticky hold) | `Scene` | "BRB on (hold). Use !auto to resume automatic switching." |
| `live` | `SetScene{switcher.scenes.normal, ttl_secs: Some(0)}` | `Scene` | "Live scene on (hold)." |
| `auto` | `SetMode{Auto}` | `Scene` | "Automatic switching on." |
| `scene <exact name…>` | `SetScene{name, ttl_secs: None}` (timed override); multiword text preserved | `Scene` | "Scene: <name> for 60 s" or "No such scene" |
| `bitrate` | read-only, from the cached snapshot | `Read` | "Bitrate 4820 kbps, RTT 85 ms (relay)" or "No current data" |
| `fix` | `RepairMedia{first media source}` | `MediaRepair` | "Repairing feed…", then "Feed repair done (unverified)" |
| scene shortcuts | `SetScene{<scene>, ttl_secs: Some(0)}` for each `chat.sceneShortcuts` entry | `Scene` | "<scene> on (hold)." |

- Scene shortcuts are migrated from legacy `optionalScenes` (`starting`, `ending`, `privacy`). They keep the legacy `!starting`, `!ending` and `!privacy` behaviour as configurable aliases (D-12).
- Unknown commands and non-command chat are ignored silently.
- Other legacy NOALBS commands survive only if the owner maps them to these intents (D-12).

**Permissions (`chat/permissions.rs`)**
- **Broadcaster:** sender ID equals the configured broadcaster ID exactly. Default capabilities: `Read`, `Scene`, `Broadcast`, `MediaRepair`.
- **Moderator:** needs **both** current provider role evidence and an entry in `chat.permissions.moderators[{provider, userId, capabilities}]`.
- **Anyone else:** `Read` only if `publicRead` is set (default `false`); `bitrate` is the only read command.
- The strongest allowed role is evaluated explicitly; there is no order-dependent badge folding. Display names are labels only.
- `Lifecycle`, `Shutdown`, `Config`, `RecoveryReset`, `Hold` and `ReleaseAdminHold` are never reachable from chat.
- **Limits:** a per-command cooldown per channel (`cooldownMs`, default 5000), and the mutation and read rates of §8.10. A sender who hits a limit is told so at most once per 30 s.

**Replay ledger (`chat/ledger.rs`)**
- Stored at `<sessionDir>/state/chat-replay.json`: mode `0600`, atomic writes, entries `(provider, broadcaster_id, message_id, expires_at)` kept 10 min, at most 4096.
- An entry is written **before** submission.
- When the ledger is full of unexpired entries, new commands are rejected; nothing is evicted.
- A message redelivered after a crash is a duplicate: the reply is "outcome unknown, send the command again".
- No command text is stored.

**Twitch adapter (`chat/twitch.rs`)**
- Uses `twitch-irc` 6.1 over TLS with the `tags` and `commands` capabilities. It may reuse connection code from the legacy `twitch.rs` (history), minus the lowercasing and PubSub.
- Messages missing `id`, `user-id`, `room-id` or `tmi-sent-ts`, or coming from shared-chat source rooms, are rejected.
- **Token:**
  - Token bytes are used exactly.
  - At startup and hourly, the token is validated against the Twitch validation endpoint, checking that login and user ID match `botLogin`/`botUserId` and that the scopes include chat read and write.
  - On a 401, refresh is serialized, the rotated tokens are persisted atomically to the `tokenSecret` and `refreshTokenSecret` files, and the adapter reconnects only after validation.
  - If refresh fails, the adapter is disabled (`chat.adapter_state=disabled`) and the broadcast is unaffected.
- **Reconnect** backs off 1 s → 60 s with jitter, rejoins only the configured channel and never replays.
- **Replies** are IRC `PRIVMSG` with a reply-parent tag.

**Kick adapter (`chat/kick.rs`)**
- **Inbound:**
  - A Unix-socket HTTP/1.1 server (`hyper` 1 with `hyper-util`) at `<runtimeDir>/webhook/chat-webhook.sock` accepts only `POST /kick/webhook`, with a body of at most 64 KiB.
  - **Before parsing JSON**, it verifies the signature over the exact raw bytes and the documented header fields (message ID, timestamp, signature), using RSA-SHA256 via the `openssl` crate and Kick's public key.
  - The key is fetched from `publicKeyUrl`, cached at `publicKeyCacheFile` and refreshed at most hourly. Tests use `publicKeyFile` instead.
  - The event type must be `chat.message.sent` v1. Duplicate event IDs are rejected.
  - A failure returns 401/400 and logs `chat.webhook_rejected` with only the reason category.
- **Outbound:** replies go through the documented Kick chat HTTP API with an OAuth access token; refresh works as for Twitch.
- **Unverified facts.** Header names, payload fields and endpoints are configuration and fixtures, verified against Kick's current documentation at implementation time (Q-08).
- **Fail closed.** If trusted key refresh or trusted time is unavailable, new mutations fail closed.

**Config (`chat` block)**

```json
"chat": { "commandPrefix": "!", "replyEnabled": true, "publicRead": false,
  "twitch": { "enabled": false, "broadcasterId": "<id>", "channelLogin": "<login>", "botLogin": "<login>", "botUserId": "<id>",
              "clientIdSecret": {"file": "…"}, "tokenSecret": {"file": "…"}, "refreshTokenSecret": {"file": "…"},
              "clientSecretSecret": {"file": "…"}, "ircEndpoint": null },
  "kick": { "enabled": false, "broadcasterUserId": "<id>", "publicKeyUrl": "<documented URL>", "publicKeyFile": null,
            "publicKeyCacheFile": "/…/state/kick-public-key.pem", "apiBaseUrl": "<documented URL>",
            "clientIdSecret": {"file": "…"}, "clientSecretSecret": {"file": "…"}, "tokenSecret": {"file": "…"}, "refreshTokenSecret": {"file": "…"} },
  "commands": { "start": {"aliases": ["start","go"], "cooldownMs": 5000}, "stop": {"aliases": ["stop","end"], "cooldownMs": 5000} },
  "sceneShortcuts": { "starting": "Starting", "privacy": "Privacy" },
  "permissions": { "broadcasterCapabilities": ["read","scene","broadcast","mediaRepair"],
                   "moderators": [ {"provider": "twitch", "userId": "<id>", "capabilities": ["read","scene"]} ] },
  "rateLimits": { "perSenderMutationsPerMin": 10, "perSessionMutationsPerMin": 60, "burst": 5, "perSenderReadsPerMin": 20 } }
```

- `ircEndpoint` (a plaintext loopback override) and `publicKeyFile` are for tests only. `check-config` rejects them unless `session.sessionId` starts with `dev`.
- Every shortcut target must be a `studio.templateScenes` value. Migrated configs carry the legacy names as extra template entries; the S09 live fixture adds `"privacy": "Privacy"` to `templateScenes`.
- **Migration rows:**

  | Legacy | Canonical |
  |---|---|
  | `chat.prefix` | `commandPrefix` |
  | `chat.commands.<cmd>.alias` | `commands.<cmd>.aliases`, only for mapped commands |
  | `optionalScenes.{privacy,starting,ending}` | `sceneShortcuts` |
  | `chat.{username, admins, ignoreUsers}` and permission rules | **not** migrated; a warning says to re-provision by stable ID |
  | `chat.platform: Kick` channel IDs | warning (Pusher IDs are not webhook identities) |
  | the `.env` keys `TWITCH_BOT_USERNAME`, `TWITCH_BOT_OAUTH` | never read; `migrate-config` prints how to create secret files |

**Events emitted:** `chat.adapter_state`, `chat.command_received` (provider, command, sender ID hash), `chat.command_rejected` (reason), `chat.reply_failed`, `chat.webhook_rejected`.

#### 7. Implementation tasks

**Unit A — gateway core (local)**
1. Write commands, permissions, limits and the ledger, plus the gateway with a fake adapter. Re-add `twitch-irc`, and add `hyper`, `hyper-util` and `sha2`.

**Unit B — Twitch (checkpoint S09-L1)**

2. Write the Twitch adapter and its token lifecycle.
3. Write `fake_twitch_irc.py`: plaintext loopback, capability negotiation, and messages injected from scripted identities (broadcaster, moderator, random user, another room).

**Unit C — Kick (checkpoint S09-L2)**

4. Write the webhook server, verification and the reply client.
5. Write `fake_kick.py`:
   - `send` signs with a test key via `openssl dgst -sha256 -sign` and posts over the Unix socket;
   - `serve-api` provides a fake chat API and key endpoint.

**Unit D — integration (checkpoint S09-L3)**

6. Wire chat into `daemon.rs` after the admin server, with independent tasks per adapter.
7. Add the snapshot chat fields: adapter states, last command outcome, rejected counts.

#### 8. Error handling and recovery

- Errors are `Chat { kind: Auth | Transport | Parse | Limit | Ledger }` and `WebhookRejected`.
- An adapter failure changes only that adapter's state and triggers its backoff. Revoked auth disables the adapter.
- A ledger I/O failure rejects new mutations (fail closed); reads keep working.
- A reply failure logs `chat.reply_failed` and does not change the receipt.
- Rejections reply with fixed text and never internal detail.

#### 9. Concurrency and lifecycle

- One task per adapter, plus the webhook server and one gateway task. All submissions go through `SessionHandle`; chat never holds a controller.
- At shutdown: stop accepting webhooks, then disconnect IRC.
- **Chat loss, floods and invalid commands never stop outputs.** Only an authorized `stop` does.

#### 10. Security and isolation

- Identities are stable provider IDs.
- Tokens are kept exact and only in secret files.
- Webhook signatures are verified on raw bytes before parsing.
- Strict size limits apply.
- Replay is prevented by the ledger plus envelope expiry.
- Chat can reach no admin, lifecycle or config capability.
- Labels and command text are sanitized before logging or replying.
- The webhook socket lives in its own directory, shared only with the edge proxy.
- Credentials are per seat.

#### 11. Local testing

- **Parsing:** aliases, shortcuts, multiword scene names, the 512-byte limit.
- **Permissions:**
  - the broadcaster vs. a spoofed display name;
  - a moderator without a whitelist entry is denied;
  - a whitelisted user without role evidence is denied;
  - `publicRead`.
- **Limits:** cooldowns and rate limits.
- **Ledger:** duplicates; persistence across restart; a full ledger rejects; the crash window.
- **Twitch:** missing tags and shared-chat rooms are rejected; the token is never lowercased (byte compare); refresh is serialized.
- **Kick:**
  - a valid signature is accepted;
  - tampered body or header, a stale timestamp, clock skew, a duplicate event or an oversize body are rejected;
  - re-serialized JSON fails verification.
- **Priority:** a flood of 1000 commands does not delay an admin `stop` (priority test against Step 05).
- **Canary:** tokens never appear in logs or snapshots.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S09-L1 | With fake Twitch IRC, the broadcaster's `!start` starts output and the sink receives. `!brb` gives a BRB hold; then `!auto`, `!scene Live`, `!privacy` (shortcut), `!bitrate` (reply shows the fake stats) and `!stop`. A random user's `!stop` is ignored. A moderator's allowed `!scene` works and their `!stop` is refused | receipts and replies match |
| S09-L2 | Fake Kick webhooks signed with the test key produce the same command flow. A tampered signature and a replayed event are rejected. Replies reach the fake API | PASS |
| S09-L3 | Killing the fake IRC server mid-broadcast sets the adapter to `reconnecting`, with the broadcast unaffected; on restart it rejoins. A 200-message flood: limits hold and the sink has no gap > 2 s | PASS |
| S09-M1 (`--manual`) | With OA-10: a real Twitch test bot in a test channel runs `!bitrate` and `!brb`. **No broadcast to a real channel**; outputs stay on the local sink | recorded in the manual checklist |

Each check asserts zero customer impact. Cleanup: stop all fakes and delete the test ledgers.

#### 13. Acceptance criteria and definition of done

1. Local tests pass.
2. Chat holds only scoped `SessionHandle`s; `tests/architecture.rs` is extended to cover `chat/`.
3. No token mangling and no unauthenticated mutation.
4. S09-L1 to S09-L3 PASS, all S01–S08 checks still PASS, and S09-M1 is either recorded or explicitly deferred by the owner.

#### 14. Handoff to later steps

- **Step 10:** the chat snapshot fields.
- **Step 11:** the webhook socket path and the secret-file layout.
- **Step 12:** the per-seat chat credentials and the edge-proxy mount contract.
- **Step 13:** the fakes and the manual checklist items.

#### 15. Open questions and assumptions

- Kick's webhook headers, signature scheme, key endpoint and chat API are external facts to verify (Q-08).
- The edge-proxy infrastructure needs approval (D-10).
- Test accounts (Q-09, OA-10).
- The command set and legacy commands, including scene shortcuts and English-only replies (D-12).
- Dropping raid auto-stop (D-17).
- Host clock synchronization (Q-18).
<!-- STEP 09 END -->

<!-- STEP 10 START -->
### Step 10 — Interactive terminal interface (`irlosd tui`)

#### 1. Title and metadata

| | |
|---|---|
| Step | 10 of 13: attach-only terminal client over the admin socket (ratatui + crossterm) |
| Depends on | 06 (`AdminClient`, subscriptions); its screens use data from 07–09 |
| Dependents | 11 (attach inside a seat), 13 (attach/detach scenarios) |
| Absorbs old steps | 14 (TUI) |
| Code status at drafting | Not started. `src/tui/mod.rs` is empty scaffolding from Step 01, and ratatui/crossterm are not yet dependencies |

#### 2. Objective

Give an operator, usually connected over `ssh GPU` or `docker exec`, a navigable live view of one session. The operator can issue the same commands as `irlosd command`, with confirmation for disruptive ones. The TUI is purely an admin-socket client. Attaching, detaching, a dropped SSH connection, a TUI crash or a terminal resize never affects the daemon or the broadcast.

#### 3. Why this step exists

The owner asked for "a tui that opens up on startup and runs headless in the background … a very simple navigable terminal window". The documented design splits that request:
- the daemon is headless (`serve`, run by init);
- the TUI is a separate attach client (`irlosd tui`), which runs whenever an operator opens a terminal.

Building it on the admin protocol guarantees it cannot become a second control path or hold a competing copy of state.

#### 4. Starting state

- **Step 06 handoff:**
  - `AdminClient` (`connect`, `status`, `submit`, `wait_receipt`, `subscribe`, `failures`);
  - JSON snapshots;
  - socket discovery: `--socket`, then `IRLOSD_ADMIN_SOCKET`, then `/run/irlos/irlosd.sock`;
  - the exit codes.
- Steps 07–09 have added telemetry, policy, media and chat fields to `SessionSnapshot`.
- `cli.rs` routes `tui`, and no-arguments-on-a-TTY, to a "not implemented" exit 2.
- S01–S09 pass.

#### 5. Scope

**In scope:**
- `src/tui/{mod,app,screens}.rs`;
- the ratatui 0.29 and crossterm 0.28 dependencies (the versions in the old plan; newest compatible patch);
- launch and discovery;
- the screens and key bindings;
- confirmation dialogs;
- terminal restoration;
- a scripted test harness;
- the live attach/detach checks.

**Out of scope:**
- Daemon changes. The only exception is a read-only, redacted snapshot field a screen needs, added under Step 05's snapshot rules.
- Streaming Studio's own logs. The TUI shows only the daemon's sanitized failure records and snapshot data.
- Remote network access.
- Web dashboards.

#### 6. Design specification

**Launch**
- `irlosd tui [--socket <path>]`. With no arguments on a TTY, `irlosd` behaves as `irlosd tui`.
- Startup: connect (`hello`), subscribe, render.
- If no daemon answers, it shows "irlosd not running at <path>" and exits 4. It **never starts a daemon**.
- If the connection is lost while running, a "disconnected — retrying" banner appears and the client reconnects with backoff (0.5 s → 5 s). It resubscribes for a full fresh snapshot and never replays old mutations.
- Inside a seat: `docker exec -it irlos-seat-<id> irlosd tui` (Step 11).

**Architecture.**
- `app.rs` holds a pure `App`: the last `SessionSnapshot`, the selected screen, focus, pending confirmations, the receipts list and the connection state.
- The event loop `select!`s over crossterm events, snapshot notifications, receipt updates and a 250 ms redraw tick, and **only redraws** on a change or a tick.
- `App` never treats its copy as authoritative. Every fact shown comes from the latest snapshot, together with its age.

**Screens** (`1`–`6`, `Tab`/`Shift-Tab`)

| # | Screen | Content |
|---|---|---|
| 1 | Overview | Session ID, epoch, engine state, uptime, Studio process state, controller state (including `UiBlocked`), current scene and mode, aggregate output state, active alerts (Degraded, CleanupIncomplete, recovery exhausted, UI blocked) |
| 2 | Stream health | Per-provider bitrate and RTT gauges with a 60-sample sparkline (history kept client-side only), quality, freshness, last-measurement age, severity, candidate count (e.g. "Low 2/3"), stale-episode flag |
| 3 | Scenes | Scene list with current and desired markers; mode (Auto, ManualOverride with countdown, Hold kind). Actions: switch (Enter, timed override), hold (`h`), release, auto |
| 4 | Outputs and recording | Per-output phase, duration, bytes, dropped frames and last-failure category. Actions: start, stop, recording on/off |
| 5 | Controller and media | Driver, connection generation, capabilities, Studio versions (informational), media sources with phase and repair state, attempts and cooldown. Action: repair |
| 6 | Chat and events | Adapter states, last commands and their outcomes (sanitized), rejected counts; paged sanitized failure records (`failures`) |

**Keys**

| Key | Action |
|---|---|
| `↑` / `↓`, `Enter` | navigate, act |
| `s` | start |
| `x` | stop |
| `b` | BRB hold |
| `h` | hold the current scene |
| `a` | auto |
| `r` | repair |
| `?` | help |
| `q` or `Ctrl-C` | quit the TUI only |

**Confirmation.**
- A modal shows the exact effect before any of these: stop broadcast, stop one output, lifecycle stop, recording off, shutdown, reset recovery, release an administrative hold.
- Stop and shutdown require typing `yes`.
- Actions the principal lacks are hidden, or greyed out with the reason shown.

**Feedback.** Each submission's receipt is shown as it moves from `Accepted` to `Applied`, `CompletedUnverified`, `OutcomeUnknown`, `Failed(code)` or `Superseded`. Success is never shown before `Applied`.

**Terminal safety.**
- Raw mode and the alternate screen are entered under a guard that restores the terminal on drop, on panic (by chaining `std::panic::set_hook`) and on SIGHUP and SIGTERM.
- Snapshot text is already sanitized. It is filtered again for control characters and truncated to cell width.
- The minimum size is 80×24. Below that the TUI shows "terminal too small" and keeps running.

**Events:** none. The TUI is a client and writes nothing into the daemon's log stream.

#### 7. Implementation tasks

**Unit A — app model and harness (local)**
1. Write the `App` reducer, the screens and key handling.
2. Write a test harness on ratatui's `TestBackend` with scripted snapshots and key sequences, saving golden buffers under `tests/fixtures/tui/`.

**Unit B — client wiring (checkpoint S10-L1)**

3. Connect through `AdminClient`, adding reconnect, confirmations and receipt tracking.
4. Add a scripted mode, `irlosd tui --script <file>`, under the `dev-tools` feature only. It feeds keys with delays and dumps frames, so live checks run without a human. For true TTY behaviour, live checks run it under `script(1)` or a pty helper.

**Unit C — live attach/detach (checkpoints S10-L2, S10-L3)**

5. Write the `live/checks/s10.sh` scenarios below.

#### 8. Error handling and recovery

- Connection errors show the banner and trigger backoff.
- A rejected command shows the `PublicFailure` code and summary.
- A protocol mismatch at `hello` exits 5 with a clear message.
- A rendering panic restores the terminal, prints a short notice and exits 1. The daemon is unaffected (S10-L3).

#### 9. Concurrency and lifecycle

- One client process, with no daemon-side state beyond its subscription.
- On quit or crash, the socket closes and the server cleans up the subscription (Step 06).
- **Detach never sends an intent.** This holds for `q`, a killed terminal, SSH loss (SIGHUP) and `kill -9` of the TUI; the daemon and the broadcast continue.
- Several TUIs may attach at once, within the 16-client limit, and all show the same truth.

#### 10. Security and isolation

- The TUI has exactly the capabilities of its peer-UID principal; it has none of its own.
- No secrets are displayed, because snapshots are redacted.
- Destructive actions require confirmation.
- Sanitization prevents terminal escape injection.
- The TUI connects only to the session socket it was pointed at; there is no discovery of other seats.

#### 11. Local testing

- **Rendering:** golden-buffer tests for every screen with normal, degraded, UI-blocked and failed snapshots; a small terminal.
- **Interaction:**
  - key navigation;
  - a confirmation is required and `Esc` cancels it;
  - capability hiding;
  - receipt progression.
- **Resilience:** reconnect after the server drops (mock admin server); a panic restores the terminal (run in a pty and check the restore sequence is emitted).
- **Sanitization:** control characters in a chat-label fixture are stripped.
- **Architecture:** `tests/architecture.rs` asserts that `tui/` imports only admin-client types and snapshot DTOs.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S10-L1 | A scripted TUI attaches to a live `serve` with real Studio and renders screens 1–6: current scene, telemetry from `fake_stats.py`, output phases. Frame dumps contain the expected fields | PASS |
| S10-L2 | During a broadcast to `rtmp_sink.sh`, the scripted TUI switches scene (Enter on `Live`), presses `b` (BRB hold), then `a` (auto); the receipts reach `Applied`. Stop requires the typed confirmation, and cancelling it leaves the broadcast running | PASS |
| S10-L3 | Detach safety mid-broadcast: quit with `q`; `kill -HUP` the TUI's pty (simulated SSH loss); `kill -9` the TUI; force a panic in the `dev-tools` build. After each, the snapshot revision keeps advancing, outputs stay `Active` and the sink has no gap > 2 s | PASS |

Each check asserts zero customer impact. Cleanup is as in earlier steps. No owner-approved actions are needed.

#### 13. Acceptance criteria and definition of done

1. Local tests pass, including the golden buffers.
2. The TUI uses only the admin client (enforced by `tests/architecture.rs`).
3. No detach path changes the session (S10-L3).
4. Disruptive actions require confirmation.
5. The terminal is always restored.
6. S10-L1 to S10-L3 PASS, and all S01–S09 checks still PASS.

#### 14. Handoff to later steps

- **Step 11:** documents `docker exec … irlosd tui` as the operator attach path and checks it inside the container.
- **Step 13:** reuses the scripted TUI for attach/detach scenarios mid-broadcast.

#### 15. Open questions and assumptions

- A separate `irlos-tui` binary versus the `irlosd tui` subcommand. The documents conflict; the latest design, the subcommand, is kept (D-20).
- Screen content beyond the coverage above is an implementation choice.
- NVENC status appears only as reported capabilities and statistics; the TUI never probes the driver.
<!-- STEP 10 END -->

<!-- STEP 11 START -->
### Step 11 — Single-session container (seat image, private display, GPU encoding)

#### 1. Title and metadata

| | |
|---|---|
| Step | 11 of 13: the `irlos-seat` image and seat init. One container runs one session (private display, irlosd, IrlosStudio) with NVENC on the GTX 1650 |
| Depends on | 01–10 |
| Dependents | 12 (multiple seats), 13 |
| Absorbs old steps | 15 (part: Dockerfile, entrypoint, display, GPU access, one seat) |
| Code status at drafting | Not started. No Dockerfile, compose file or unit exists in either repo. Docker and the NVIDIA Container Toolkit are assumed absent on `ssh GPU` (OA-6) |

#### 2. Objective

Package one complete session as a container. It:
- starts a private display;
- runs `irlosd serve`, which alone launches IrlosStudio;
- encodes with NVENC through the NVIDIA Container Toolkit;
- receives its configuration, secrets and SRT/telemetry identifiers from a mounted seat volume, never from the image;
- shuts down within its grace period, with no orphan processes.

#### 3. Why this step exists

The owner's model is "one person per instance", containerized (Docker/Compose was chosen in planning). It replaces today's practice of running every customer as root on one shared display with one shared config tree and one shared obs-websocket password (RECON). A correct single seat must exist before isolation between seats can be proved (Step 12). The image also makes the IrlosStudio build reproducible: its build stage is the Route A dev box stage (D-09).

#### 4. Starting state

- **Steps 01–10 complete:**
  - `irlosd` with `serve`, `status --health`, `command` and `tui`;
  - the supervisor's launch spec, environment, templates and `studio.networkNamespace` (04);
  - UI-blocked detection (05);
  - the webhook socket (09).
- The IrlosStudio build procedure and the `deploy/devbox/Dockerfile` (Route A, Step 01).
- The measured NVENC session limit (S04-L4).
- All S01–S10 checks pass on the chosen route.
- **OA-6** (Docker Engine, the NVIDIA Container Toolkit with its runtime configured, and dev-user Docker access or rootless Docker) must be approved before this step's live checks.

#### 5. Scope

**In scope:**
- `deploy/seat/Dockerfile` (multi-stage);
- `deploy/seat/seat-init.sh`;
- `deploy/xorg/` (template for the `nvidia_xorg` profile only);
- `deploy/seat/seat.compose.yml` (single-seat template);
- `deploy/README.md`;
- `live.sh build image`;
- the health check, signals and grace period;
- a read-only root filesystem and the mounts;
- per-seat resource limits for one seat;
- per-seat resource measurement.

**Out of scope:**
- Multiple seats, cross-seat UID mapping, seat-to-seat network isolation and `seatctl` (Step 12).
- The Kick edge proxy (deployment; D-10).
- Host hardening beyond what one seat needs (Step 12).

#### 6. Design specification

**Image (`deploy/seat/Dockerfile`)**

| Stage | Contents |
|---|---|
| `studio-build` | **The same stage as the dev box**: Ubuntu 24.04 plus the CI dependency list. Builds IrlosStudio at the synced commit (`-DENABLE_BROWSER=OFF`, `OBS_VERSION_OVERRIDE`, obs-websocket and `irlos-control` enabled) into `/opt/irlosstudio` |
| `irlosd-build` | Pinned Rust toolchain; `cargo build --locked --release` (no `dev-tools`) |
| `runtime` | Ubuntu 24.04 minimal plus IrlosStudio's runtime libraries: FFmpeg 6.1 shared libraries (`libavcodec60` and the rest), Qt 6 with the xcb platform plugin, X client libraries. Also `xvfb`, `xauth`, `tini`, CA certificates, and `/opt/irlosstudio` and `/usr/local/bin/irlosd`. No compilers, no ffmpeg CLI, **no secrets or configs**. Templates are embedded in the binary |

- Tag: `irlos-seat:<irlosd-version>-<studio-short-commit>`.
- `USER` is a fixed unprivileged UID/GID (default `10001:10001`; Step 12 overrides it per seat). Studio and irlosd run as the same UID (D-05).
- NVIDIA user-space libraries are **not** baked in. The toolkit injects them at run time, matching the host driver.

**Seat init (`deploy/seat/seat-init.sh`, run under `tini` as PID 1)**
1. Validate mounts and write nothing outside them:
   - the session dir is writable, owned by the seat UID, mode `0700`;
   - `/run/irlos` is a private tmpfs;
   - `/run/irlos/webhook/` is the only optional shared mount;
   - the config file has mode `0600` and declares `studio.networkNamespace: "private"`.
2. Start the display for `IRLOS_DISPLAY_PROFILE`:
   - **`software_xvfb`** (the default, and the profile used on `ssh GPU`): `Xvfb :1 -screen 0 1920x1080x24 -nolisten tcp -auth /run/irlos/Xauthority`. The cookie comes from `xauth`; there is no `-ac`, and the container keeps its own `/tmp/.X11-unix`, never the host's.
   - **`nvidia_xorg`** (the production profile to qualify): a private Xorg with the NVIDIA driver and a headless config from `deploy/xorg/`. It needs the `graphics,display` capabilities and a matching host X driver module. Qualify it only on hardware the owner designates (D-07, Q-12), with no silent fallback to software.
3. Wait for the display to be ready (socket plus `xdpyinfo`), then log `seat.display_ready`.
4. Start `irlosd serve --config <path>` as a child. If either the display or irlosd exits, stop the other: SIGTERM to irlosd, wait ≤ 22 s, then SIGKILL. Log `seat.exit` and exit with irlosd's code; Docker's restart policy applies.
5. Forward SIGTERM/SIGINT from tini to irlosd. irlosd performs the §8.6 shutdown and SIGINTs Studio itself.

**Run contract (`deploy/seat/seat.compose.yml`, single seat; the equivalent `docker run` is in `deploy/README.md`)**

| Setting | Value |
|---|---|
| GPU | `runtime: nvidia` (or `--gpus device=<uuid>`) with `NVIDIA_VISIBLE_DEVICES=<selected uuid>`. `NVIDIA_DRIVER_CAPABILITIES=compute,video,utility` for `software_xvfb`, because the NVENC soft path uses CUDA (Step 04). Add `graphics,display` only for `nvidia_xorg` |
| Filesystem | `read_only: true`; tmpfs `/run/irlos` (`mode=0700,uid=<seat uid>`); size-limited tmpfs `/tmp`; seat volume bound to `/var/lib/irlos/sessions/<id>` |
| Security | `cap_drop: [ALL]`, `security_opt: [no-new-privileges:true]`, default seccomp and AppArmor, `pids_limit: 512`, `ulimits.core: 0` |
| Resources | `mem_limit` and `cpus` from measured use plus 20% headroom (S11-L4, development hardware) |
| Network | A bridge network of its own; **no published ports**; outbound traffic only (enforced in Step 12). obs-websocket, if used, binds inside this namespace only |
| Process | `init: false` (tini is already PID 1); `stop_signal: SIGTERM`; `stop_grace_period: 25s`; `restart: on-failure` |
| Health | `irlosd status --health --socket /run/irlos/irlosd.sock`; interval 10 s; start period 90 s |
| Env | `IRLOS_DISPLAY_PROFILE` and `IRLOSD_LOG` only; **no secrets in env** |

**Inputs.** Everything arrives through the volume. The image is identical for every seat.

| Path in the volume | Contents |
|---|---|
| `irlosd.json` | `paths.runtimeDir=/run/irlos`, `paths.sessionDir=/var/lib/irlos/sessions/<id>`, `studio.networkNamespace="private"` |
| `secrets/` | stream keys; the ingest URL, whose stream id is the relay's only secret; telemetry publisher/URL secrets; chat tokens |
| `assets/` | the BRB card |
| `state/` | the chat ledger |
| `studio/` | generations, including Studio's own logs, which are secret-bearing (Step 04) |

**Operator attach.** `docker exec -it irlos-seat-<id> irlosd tui` or `irlosd status`. The exec runs as the seat UID, so it is the `LocalService` principal (D-05).

#### 7. Implementation tasks

**Unit A — image (checkpoint S11-L1)**
1. Write the seat Dockerfile, reusing the dev box build stage.
2. Add `live.sh build image`. It builds on `ssh GPU` and tags `irlos-seat:dev-<commit>`.
3. Add image checks: no canary strings in the layers (`docker save | tar -xO | grep -c <canary>`), the expected `USER`, and no compilers.

**Unit B — seat init and single seat (checkpoints S11-L2, S11-L3)**

4. Write `seat-init.sh` and the compose template.
5. Write the `live/checks/s11.sh` fixture generator. It creates `~/irlos-dev/seats/dev1/volume/` with a fake config and secrets, under the compose project `irlos-dev-dev1`.
6. Run the SRT source and RTMP sink as companion containers (`irlos-dev-fakes-*`) on a dedicated test network that only the seat and the fakes join. Nothing binds host interfaces.

**Unit C — measurement (checkpoint S11-L4)**

7. During a 10-minute broadcast to the sink, record:
   - per-seat VRAM and encoder use from `nvidia-smi pmon -c 1 -s um`, sampled every 10 s;
   - container CPU and RSS from `docker stats --no-stream`;
   - Studio render and output frame statistics from `get_stats`.

   Label everything development hardware.

#### 8. Error handling and recovery

- Seat init exits nonzero on invalid mounts or permissions before starting anything.
- A display failure stops the seat; there is no Studio without a display.
- An irlosd failure stops the seat, and Docker restarts it. The cold start leaves desired outputs `Stopped` (no auto-resume).
- An unhealthy check is reported; irlosd does not act on it.
- If NVENC is unavailable in the container, encoder creation fails, the output start fails with `unavailable` and the seat stays up. **There is no silent x264 fallback.**
- A UI-blocked Studio inside the seat is handled as in Step 05 (D-25).

#### 9. Concurrency and lifecycle

- The ownership chain is tini → seat init → {display, irlosd → Studio}.
- `docker stop` sends SIGTERM to tini, which forwards it. irlosd shuts down in ≤ 20 s and seat init stops the display; the 25 s grace period covers both.
- `docker kill` tears down the cgroup, so orphans cannot survive.
- `docker exec` attach and detach never affect the session (S11-L3).

#### 10. Security and isolation

- Unprivileged UID; all capabilities dropped; `no-new-privileges`; read-only root; private tmpfs for the runtime dir.
- No host `:0` and no host X sockets. No Docker socket mounted.
- Secrets only in the volume, mode `0600`, never in env or the image.
- No published ports.
- Only the selected GPU is visible, with minimal driver capabilities.
- `deploy/README.md` must state that a container is a shared-kernel, shared-GPU boundary, not protection against driver flaws, and that installing Docker changes host networking (OA-6).

#### 11. Local testing

The local machine has no Docker, so local checks are static:
- `shellcheck` of `seat-init.sh`, if available user-locally;
- unit tests of the fixture generator;
- a compose-template rendering test.

The runtime behaviour is tested live.

#### 12. Live testing on `ssh GPU`

Requires OA-6, plus OA-14 if customer workloads are present.

| Check | Proves | Expected |
|---|---|---|
| S11-L1 | The image builds on `ssh GPU`, runs as the seat UID, contains no canary secret, and `irlosd --version` and `obs --version` work inside it | PASS |
| S11-L2 | `docker compose up` of seat `dev1`: healthy within 90 s; `docker exec … irlosd status --json` shows engine `Ready` and the BRB scene; `nvidia-smi -L` inside lists only the selected GPU | PASS |
| S11-L3 | Broadcast inside the seat: `docker exec … irlosd command start` → the sink container receives ≥ 60 s of H.264 (`ffprobe` on the file). `pmon` shows an `enc` value for the seat's Studio process. Scripted `irlosd tui` attach and detach mid-broadcast cause no gap > 2 s. `docker stop` completes in < 25 s with exit 0, leaving no processes or GPU contexts | PASS; customer impact zero |
| S11-L4 | 10-minute per-seat measurement recorded (development hardware). After `docker kill` and a restart, desired outputs are `Stopped` | PASS |

Cleanup: `docker compose down -v` for `irlos-dev-*` projects only, remove the fakes containers, and keep at most the last two dev images.

#### 13. Acceptance criteria and definition of done

1. The image contains no secret or customer data.
2. The seat runs unprivileged under the §6 run contract.
3. NVENC encoding is verified inside the container (S11-L3).
4. Shutdown completes within the grace period with no orphans.
5. Attach and detach are safe.
6. S11-L1 to S11-L4 PASS, and all S01–S10 checks still PASS. Development keeps using the dev box or native route.
7. `deploy/README.md` documents the run contract, the display profiles and the limits of container isolation.

#### 14. Handoff to later steps

- **Step 12:** the image and tag scheme; `seat-init.sh`; the single-seat compose template; the volume layout; the measured per-seat resources; the health check.
- **Step 13:** a reproducible seat for scenarios and soaks.

#### 15. Open questions and assumptions

- Docker versus the systemd template units in IrlosWeb's plan (D-06). This roadmap keeps Docker, the owner's planning choice.
- The Docker install and access model on `ssh GPU` (OA-6, D-06).
- `nvidia_xorg` feasibility (D-07, Q-12).
- The seat UID (D-05, Q-14).
- Where production images come from (Q-11).
- Base-image licensing and the GPL source offer for shipped images (Q-21).
<!-- STEP 11 END -->

<!-- STEP 12 START -->
### Step 12 — Multi-tenant isolation and seat lifecycle

#### 1. Title and metadata

| | |
|---|---|
| Step | 12 of 13: several isolated seats on one GPU host; `deploy/seatctl`, whose vocabulary matches the IrlosWeb GPU-host agent contract; isolation and capacity tests on development hardware |
| Depends on | 11 |
| Dependents | 13; the future IrlosWeb agent (outside this roadmap) |
| Absorbs old steps | 15 (rest: multi-seat compose, isolation, resource limits, lifecycle) |
| Code status at drafting | Not started. IrlosWeb's agent task (`TASK_1_GPU_AGENT.md`) has not been executed |

#### 2. Objective

Run as many seats as the GTX 1650 actually supports side by side, and prove that none can see or affect another's files, sockets, display, processes, network, credentials or GPU contexts.

Make seat lifecycle one auditable tool, `seatctl`. Its commands and JSON outputs match the frozen agent contract (`IrlosWeb/tasks/CONTRACT_AGENT_API.md`), so the planned agent becomes a thin wrapper.

Record per-seat resource use and the measured seat ceiling for this hardware. Full 8-seat validation is pending production hardware.

#### 3. Why this step exists

IRLOS puts mutually distrustful customers on one GPU server. Today that server runs everyone as root, on one display, with one config tree and one obs-websocket password (RECON). The isolation contract must be enforced and tested, not inferred from the word "container":
- users and filesystems;
- sockets and credentials;
- network and resource limits;
- the host's responsibilities.

Provisioning must also stop being hand work. IrlosWeb already froze an ICP-shaped agent vocabulary: `provision`, `suspend`, `destroy`, `status`, `capacity`, `control` with `start`/`stop`/`set_scene`/`get_state`. Matching it lets "irlosd replace the internals without the web app noticing" (`PROVISIONING_SPEC.md` §7).

#### 4. Starting state

- **Step 11 handoff:**
  - the `irlos-seat` image and tag scheme;
  - `seat-init.sh` and the single-seat compose template;
  - the run contract and volume layout;
  - the health check;
  - per-seat measurements (S11-L4).
- The measured NVENC limit (S04-L4).
- S01–S11 pass.
- **Owner approvals:**
  - **OA-12** (per-seat UIDs via host users or Docker user-namespace remapping) before the isolation checks;
  - **OA-13** (host firewall rules for egress allowlists) before the network checks;
  - **OA-14** for multi-seat GPU load while customer workloads exist.

#### 5. Scope

**In scope:**
- `deploy/seatctl`, one dependency-free Python 3 (stdlib) script;
- `deploy/seat/seat.compose.yml.tmpl`, rendered per seat;
- `deploy/host/` documentation and templates: the firewall rule template, seat UID allocation, log rotation, Docker daemon settings, the edge-proxy mount for the Kick webhook directory;
- per-seat networks and resource limits;
- capacity admission;
- upgrades;
- suspend and secure destroy;
- reconciliation;
- the isolation test suite.

**Out of scope:**
- The IrlosWeb agent's HTTP server, WireGuard, Stripe and the web app (IrlosWeb TASK_1–3).
- The Kick edge-proxy implementation.
- Production host hardening beyond templates.
- VM, vGPU or dedicated-GPU isolation, documented as the option for stronger guarantees.

#### 6. Design specification

**Seat identity and layout**
- The seat ID is the `SessionId`. The agent's slug (`^[a-z0-9]{6}$`) is a valid `SessionId`, and `seatctl` uses it unchanged.
- The relay stream id is a separate, long, random secret, delivered as a file in `--secrets`. It is never derived from the slug: RECON shows the relay has no auth and the first publisher wins.
- **UIDs.** Each seat gets a unique UID/GID: production `20000 + n`, development `21000 + n`.
- **Paths.** Seats live under `<seatsRoot>` (production `/var/lib/irlos/seats`, development `~/irlos-dev/seats`). Each seat directory holds `volume/` (mounted as the session dir), `webhook/` (the edge-proxy mount), `seat.json` and `compose.yml`.
- `seat.json` records `{sessionId, tenantId, uid, gid, image, gpuUuid, createdAt, state}` and holds no secrets.
- The container runs with `user: "<uid>:<gid>"` and a tmpfs owned by that UID.
- Directory ownership changes need root and go through OA-12: host `useradd`/`chown`, or Docker `userns-remap`. Docker-group access is root-equivalent, and `deploy/README.md` says so.

**`seatctl` commands.** JSON output uses the agent contract shapes. Errors are `{"error": "<code>", "detail": "<text>"}` with the codes `bad_slug`, `at_capacity`, `not_found`, `not_ready`, `unauthorized` and `internal`.

| Command | Effect | JSON result |
|---|---|---|
| `provision <slug> --tenant <tid> --config <seed.json> --secrets <dir> [--image <tag>] [--gpu <uuid>]` | Validate the slug, run admission, allocate the UID, create the volume (`secrets/`, `assets/`, `state/`, `studio/`, all `0700`), **copy** secret files as `0600` without printing them, render `irlosd.json` with `sessionId=<slug>` and seat paths, run `irlosd check-config` inside the image, render compose, create network `irlos-net-<slug>`, start, and poll `status` until ready (45 s). Idempotent: an existing running seat returns its status. On failure, everything this command created is rolled back | `{"slug", "status": "ready"\|"starting"\|"failed", "studio_port": null, "display_num": null}` |
| `status <slug>` | Read the container state, plus `irlosd status --json` through `docker exec` | `{"slug", "status", "units": {"xvfb", "encoder", "bitrate_manager"}, "uptime_s", "streaming", "bitrate_kbps", "scene", "scenes", "ingest_connected"}` |
| `control <slug> <action> [json-args]` | `start` / `stop` / `set_scene {scene}` / `get_state`, mapped to `irlosd command start` / `stop` / `scene <name>` / `status`. `set_bitrate` returns `{"error":"internal","detail":"unsupported: encoder bitrate is fixed (D-11)"}` | `{"ok": true, "state": <status shape>}` |
| `suspend <slug>` | Graceful stop, keep the data, free GPU resources, mark suspended (no restart policy) | `{"slug", "status": "suspended"}` |
| `resume <slug>` / `restart <slug>` | Start or restart a suspended or running seat, after admission | status shape |
| `destroy <slug> --yes [--keep-volume]` | Stop, remove the container and network, securely delete `secrets/` (overwrite, then unlink) and the volume, release the UID. Idempotent: already gone is success | `{"slug", "status": "destroyed"}` |
| `upgrade <slug> --image <tag> [--allow-active]` | Refuse if any output is `Active` or pending, unless the owner-only `--allow-active` is given (the broadcast drops). Otherwise stop, recreate with the new image and start. After any upgrade, desired outputs are `Stopped` (cold start). Rolling upgrades go one seat at a time | status shape |
| `capacity` | Run admission (below) and report | `{"host", "tenant_max", "tenants_active", "slots_free", "vram_total_mb", "vram_used_mb", "vram_mb_per_tenant"}` |
| `list` / `reconcile` | List seats; compare seat directories with running containers and **report** orphans, never auto-deleting | arrays |

- The generic unit names are part of the contract (`xvfb` = the display, `encoder` = IrlosStudio, `bitrate_manager` = the irlosd engine). The real component names never appear in `seatctl` output.
- `studio_port` and `display_num` are `null`, because ICP has no port and the display is private. Whether this needs a contract amendment is Q-17 and D-06.
- **Audit and locking.**
  - Every action appends one JSON line `seatctl.action {action, slug, result, ts}` to `<seatsRoot>/audit.log`, with no secrets.
  - Failures exit nonzero.
  - Invocations are serialized by `flock` on `<seatsRoot>/.seatctl.lock`.
- **Single path.** Only `seatctl`, or the IrlosWeb agent through it, changes seat lifecycle on a host. Ad-hoc `docker run` and systemd units are not part of the model (D-06).

**Isolation enforcement**

| Boundary | Enforcement |
|---|---|
| Filesystem | separate volume per seat, owned by the seat UID with mode `0700`; no parent `seats/` mount; read-only image; tmpfs runtime |
| Sockets | each seat's `/run/irlos` is its own tmpfs; no cross-seat bind mounts; only `<seat>/webhook` is bind-mounted, for the host edge proxy (mode `0750`, proxy group) |
| Display | a private Xvfb or Xorg per seat in its own IPC namespace; never the host's `/tmp/.X11-unix` |
| Processes | separate PID and IPC namespaces; no host PID; no privileged mode; `pids_limit` |
| Network | one bridge per seat (Docker isolates distinct bridges); no published ports; an egress allowlist via `DOCKER-USER` rules rendered from `deploy/host/firewall.tmpl` (OA-13), allowing DNS, NTP, the configured relay host and port, and the platform RTMP/IRC/API endpoints, and denying host metadata and admin networks |
| Credentials | per-seat secret files only; nothing in env, the image or the audit log |
| Resources | `mem_limit`, `cpus`, `pids_limit` and tmpfs sizes from S11-L4 plus 20% headroom |
| GPU | `NVIDIA_VISIBLE_DEVICES=<uuid>`. There is no VRAM or NVENC quota, so admission is the guard |

`deploy/host/README.md` must record three limits:
- A shared telemetry endpoint is not tenant-safe merely because irlosd filters records. It needs a seat-scoped endpoint or a filtering proxy (D-13).
- obs-websocket must never be enabled in a seat that shares a network namespace.
- The residual shared risks: kernel, GPU driver, GPU reset, NVENC contention and host operators.

**Admission (`capacity`, run by `provision`, `resume` and `restart`)**
- Inputs:
  - measured per-seat VRAM, from the S11-L4 `pmon -s um` figures;
  - the measured NVENC session limit (S04-L4);
  - current host usage from `nvidia-smi pmon` and `--query-gpu=memory.used,memory.total`, never `encoder.stats.sessionCount`;
  - a configured headroom, default 20%.
- `tenant_max` is the lower of two limits:
  - the encoder-session limit minus the sessions in use;
  - `floor((usable VRAM − headroom) / vram_mb_per_tenant)`, leaving one slot free, as in IrlosWeb `PROVISIONING_SPEC.md` §3.
- Starting a seat beyond `tenant_max` fails with `at_capacity` (exit 7) rather than overcommitting.
- On `ssh GPU` the report is labelled development hardware and counts customer usage as in-use.

#### 7. Implementation tasks

**Unit A — tool and templates (checkpoint S12-L1)**
1. Write `seatctl`, the compose template and the `deploy/host/` templates and README.
2. Add tests: template rendering, argument validation, JSON output shapes against the contract examples, and rollback. Tests use a mock `docker` shim on `PATH` that records calls.

**Unit B — two seats and isolation (checkpoints S12-L2, S12-L3)**

3. Request OA-12 and OA-13. Provision seats `dev001` and `dev002` with fake secrets and per-seat fake sinks.
4. Write the `live/checks/s12.sh` isolation probes. They run inside seat A through `docker exec` and try to reach seat B's resources. They use only shell builtins and `irlosd`, from a test-only image variant (`irlos-seat:dev-<commit>-probe`, built with `dev-tools`) whose `irlosd probe` subcommand attempts socket, file and network access. Release images never contain it.

**Unit C — capacity on development hardware (checkpoint S12-L4)**

5. Provision seats one at a time while admission allows. Broadcast to sinks from every seat.
6. Record per-seat and host metrics for 15 minutes (`pmon -s um`, `docker stats`, `get_stats`).
7. Stop at the ceiling and verify that the next `provision` returns `at_capacity`.

#### 8. Error handling and recovery

- `seatctl` aborts at the first failed step and rolls back what this invocation created, which it tracks in `seat.json`. It never deletes resources it did not create.
- A failed upgrade leaves the seat stopped, with the previous tag recorded for `upgrade --image <previous>`.
- Firewall template errors are reported. Host rules are never changed without OA-13.
- Restart storms are bounded by Docker's backoff and irlosd's own budgets. `status` shows restart counts.

#### 9. Concurrency and lifecycle

- Seats are independent: stopping, upgrading or crashing one never touches another (S12-L3).
- `seatctl` invocations are serialized by the host lock.
- **Only `seatctl suspend`, `restart`, `upgrade` or `destroy`, or an authorized in-seat intent, can stop a seat's broadcast.** `upgrade` refuses active seats by default.

#### 10. Security and isolation

This step is about isolation. In addition to the enforcement table:
- No customer gets Docker or shell access.
- The audit log contains no secrets.
- Destroy deletes secrets securely.
- `docker inspect` output, which shows mounts and env, is never logged.
- The stream id stays a secret file, and `seatctl` output never shows it.

#### 11. Local testing

- Argument validation, template rendering and JSON shapes, as snapshot tests matching `CONTRACT_AGENT_API.md`.
- Admission arithmetic, as table tests with fake metrics.
- Rollback logic and idempotent operations, using the mock docker shim.
- No secrets in the audit log or rendered compose (canary).
- `reconcile` reports orphans and never deletes them.

#### 12. Live testing on `ssh GPU`

Requires OA-6, OA-12 and OA-13, plus OA-14 if customer workloads are present.

| Check | Proves | Expected |
|---|---|---|
| S12-L1 | Round trip for one seat: `provision`, `status`, `control start`, `get_state`, `control stop`, `suspend`, `resume`, `destroy`. JSON shapes match the contract, audit lines are written, and `destroy` removes everything this run created | PASS |
| S12-L2 | Two seats broadcasting to separate sinks, each with its own UID, network and display; both healthy. `suspend dev001` leaves dev002's broadcast continuous, and `upgrade dev002` refuses while active | PASS |
| S12-L3 | Isolation probes from dev001 against dev002 all fail: reading its volume or secrets; connecting to its `irlosd.sock` or `obs-control.sock`; reaching its X display; listing or signalling its processes; connecting to its container IP; egress to a non-allowlisted address. A memory hog in dev001 is OOM-killed within dev001's limit while dev002 is unaffected | all denied; dev002 sink continuous |
| S12-L4 (`--pending-hardware` for 8 seats) | Development-hardware capacity: the maximum number of admitted seats, per-seat VRAM and encoder use, frame drop rates and CPU are recorded, and the next `provision` returns `at_capacity` | figures recorded; 8-seat validation pending production hardware |

Every check asserts zero customer impact. Cleanup: `seatctl destroy` of every `dev*` seat. Revert or keep the firewall rules as the owner directed with OA-13.

#### 13. Acceptance criteria and definition of done

1. Every isolation probe in S12-L3 is denied, and the other seat's broadcast is unaffected.
2. `seatctl` operations are idempotent, audited and roll back cleanly, and their JSON matches the agent contract (or each divergence is listed under D-06/D-11).
3. Admission prevents overcommit.
4. Capacity figures are recorded as development hardware, with full capacity explicitly pending.
5. S12-L1 to S12-L4 PASS (S12-L4 with its pending tag), and all S01–S11 checks still PASS.

#### 14. Handoff to later steps

- **Step 13:** `seatctl`, multi-seat fixtures, the isolation probes, the admission report and capacity figures.
- **The IrlosWeb agent:** can wrap `seatctl` one-to-one (Q-17).

#### 15. Open questions and assumptions

- Docker seats versus the systemd template units in IrlosWeb's plan, including the `studio_port`/`display_num` fields (D-06, Q-17).
- `set_bitrate` is unsupported (D-11).
- UID allocation and the user-namespace choice (D-05, Q-14, OA-12).
- Egress allowlist contents per deployment, which are provisioning data (Q-19).
- A seat-scoped telemetry endpoint (D-13).
- The 8-seat target needs production hardware (D-08).
- Whether production keeps sharing a GPU host with legacy customer instances during migration (Q-02).
<!-- STEP 12 END -->

<!-- STEP 13 START -->
### Step 13 — End-to-end verification and alpha release validation

#### 1. Title and metadata

| | |
|---|---|
| Step | 13 of 13: scenario matrix, fault injection, soak, manual platform checklist, release criteria and packaging of `irlosd` 0.1.0-alpha |
| Depends on | 01–12 |
| Dependents | none (release) |
| Absorbs old steps | 16 (end-to-end simulation testbed and verification runbook) |
| Code status at drafting | Not started |

#### 2. Objective

Prove on `ssh GPU`, in bare-process mode and inside seat containers, that the complete daemon behaves correctly end to end:
- normal streaming, degradation and recovery;
- feed loss and crashes;
- controller loss and a UI blocked by a dialog;
- the OBS v5 fallback;
- chat, admin and TUI use mid-broadcast;
- config reload;
- concurrent seats.

The proof uses deterministic test doubles, injected faults and a long soak. Record the evidence in `live/WALKTHROUGH.md`, complete a manual checklist for anything needing real platforms, and decide the alpha release against explicit criteria.

#### 3. Why this step exists

Each earlier step proved its own behaviour plus regression. Interactions show up only when everything runs together under realistic timing and failure. Examples:
- the policy against holds during a Studio crash;
- chat floods during shutdown;
- reconnects during a repair;
- seats competing for NVENC alongside customer workloads.

The owner's rule applies to the release: "Done means you ran it and can show the log line". Recorded evidence is required, not claims.

#### 4. Starting state

- Steps 01–12 complete. All S01–S12 checks pass on `ssh GPU`; S12-L4 is tagged pending hardware.
- Fakes and helpers available:
  - `mock_icp`, `icp_testserver`;
  - `srt_source.sh`, `rtmp_sink.sh`, `fake_stats.py`, `fake_twitch_irc.py`, `fake_kick.py`;
  - the scripted TUI;
  - `seatctl` and the seat image;
  - `live/lib/admin.sh`.
- The S04-L5 and S05-L4 dialog outcomes are recorded.

#### 5. Scope

**In scope:**
- `live/scenarios/`: declarative scenario files, plus the runner `live/lib/scenario.sh`;
- `live/lib/faults.sh`;
- `live/checks/s13.sh`;
- `live/soak.sh`;
- `live/WALKTHROUGH.md` (evidence);
- `live/MANUAL_PLATFORM_CHECKLIST.md`;
- `RELEASE_CHECKLIST.md` at the repository root;
- release packaging: a tagged seat image, a binary tarball with a sanitized example config, a `CHANGELOG.md` entry and a GPL source pointer.

**Out of scope:**
- New features. A defect found here is fixed in the owning step's modules, with that step's tests and live checks updated and re-run.
- Production rollout and customer migration.
- Full 8-seat capacity (pending production hardware).

#### 6. Design specification

**Scenario files.** `live/scenarios/<name>.json` holds ordered, timed actions and assertions:

```json
{ "name": "low_bitrate_recovery", "seats": ["dev1"], "driver": "unix_socket",
  "telemetry": {"freshnessContract": "current_on_request", "freshnessJustification": "test-only fake stats server"},
  "steps": [ {"at": 0,   "do": "stats.curve", "args": {"bitrate": 5000, "rtt": 80}},
             {"at": 5,   "do": "cmd", "args": ["start"]},
             {"at": 20,  "do": "stats.curve", "args": {"bitrate": 200}},
             {"at": 30,  "expect": "snapshot", "args": {"scene": "BRB", "outputs_active": true, "within_s": 6}},
             {"at": 40,  "do": "stats.curve", "args": {"bitrate": 4000}},
             {"at": 50,  "expect": "snapshot", "args": {"scene": "Live", "within_s": 8}},
             {"at": 60,  "expect": "sink", "args": {"continuous": true, "max_gap_s": 2}} ] }
```

- **Actions:**
  - telemetry: `stats.curve`, `stats.fault`;
  - media source: `srt.stop`, `srt.start`;
  - control: `cmd …`, `chat.twitch`, `chat.kick`, `tui.script`;
  - faults: `fault.kill_studio`, `fault.kill_irlosd`, `fault.stop_studio` (SIGSTOP for N s, then SIGCONT), `fault.rtmp_refuse` (sink not listening), `fault.config_reload`;
  - seats: `seat.provision`, `seat.suspend`, `seat.resume`, `seat.destroy`.
- **Assertions:** snapshot predicates; sink continuity and duration; receipts; the **absence** of forbidden events (for example, no `engine.output_changed` to `Stopped` without a stop intent); presence of named log events; zero customer impact.
- **Isolation of runs.** Every scenario starts from clean seats or sessions and ends with cleanup.

**Scenario matrix.** These are the minimum. Each runs in bare-process mode (dev box or native) and inside seats unless noted.

| # | Scenario | Key expectations |
|---|---|---|
| E01 | Normal streaming, 10 min | outputs Active throughout; scene `Live`; no restarts; sink continuous |
| E02 | Low bitrate and recovery | BRB/Low after 3 bad samples; `Live` after 5 fresh good samples; outputs unaffected; timings recorded |
| E03 | Feed loss and return (SRT source killed, stats `absent`) | immediate BRB on absence; media repair only once ingest is healthy again (via `obs_v5` in a private namespace, else `unsupported`); return to `Live` |
| E04 | Stale telemetry (fake stats stalls) | stale-data BRB once; with `ResponseOnly` freshness, never an automatic recovery |
| E05 | IrlosStudio crash (`kill -9`) | relaunch with backoff; outputs desired `Stopped` (no resume by default); with `resumeBroadcastAfterStudioCrash=true`, the same-epoch resume rule |
| E06 | ICP stall without a crash (Studio SIGSTOP 8 s) | controller timeouts → `Degraded`; no Studio restart within 30 s; recovery on SIGCONT; no mutation replay |
| E07 | UI blocked by a dialog (`fault.rtmp_refuse`, then start) | behaviour matches S05-L4; `engine.studio_ui_blocked`; recovery by an authorized lifecycle cycle; with D-25's opt-in, one automatic cycle |
| E08 | Fallback driver: E01–E03 with `software.type=obs_v5` in a private namespace | identical observable behaviour; media repair works |
| E09 | Chat commands mid-broadcast (Twitch and Kick fakes), including unauthorized senders, floods and replays | correct receipts and replies; a flood does not delay an admin stop |
| E10 | Admin and TUI attach/detach mid-broadcast (16 clients, a slow client, TUI `kill -9`, simulated SSH loss) | no sink gap > 2 s; daemon unaffected |
| E11 | Config reload: an allowed change (thresholds) is applied; a forbidden one (destination) is refused while active | new config generation; outputs unaffected |
| E12 | irlosd crash inside a seat (`kill -9` irlosd), seat only | seat init stops the seat and Docker restarts it; cold start with outputs `Stopped`; no orphan Studio |
| E13 | Concurrent seats (as many as admitted): E02 in one, E05 in another, seats only | no cross-seat effect |
| E14 | Seat lifecycle via `seatctl` (`provision` → `control start` → `suspend` → `resume` → `upgrade` while stopped → `destroy`), seats only | contract-shaped JSON; idempotent repeats; audit lines; nothing left behind |
| E15 | Shutdown under load (chat flood plus a pending start) | shutdown < 20 s; never `CleanupIncomplete`; exit 0 |

**Faults (`live/lib/faults.sh`).** Each fault has a matching recovery action and a cleanup trap:
- Studio KILL and STOP/CONT; irlosd KILL;
- fake stats faults;
- SRT source stop/start;
- RTMP refuse;
- a full seat tmpfs (the ledger must fail closed);
- clock skew for Kick webhooks (fake timestamps).

**Soak (`live/soak.sh`)**
- **Duration and window.** At least 60 minutes, which requires OA-14.
- **Load.** Every admitted seat broadcasts to its sink. Telemetry follows a seeded random bitrate curve. One seat's Studio is killed every 20 minutes, and chat commands arrive every 2 minutes.
- **Recorded:**
  - per-seat VRAM high-water and encoder use (`pmon -s um` every 10 s);
  - CPU and RSS;
  - Studio render/output skipped-frame ratios (`get_stats`);
  - sink gaps and restart counts;
  - irlosd RSS growth: less than 10% after warm-up is the leak check;
  - customer impact.
- **Default thresholds** (old plan, architecture report §4.4): no OOM or encoder-allocation failure; ≤ 0.1% missed render/encode frames per seat at steady state; ≥ 20% resource headroom.
- Every figure is labelled development hardware.

**Manual platform checklist (`live/MANUAL_PLATFORM_CHECKLIST.md`)**

| # | Item | Approval |
|---|---|---|
| 1 | A real Twitch test bot in a test channel: token validation and refresh, `!bitrate`, `!brb`, the moderator whitelist, outputs still on the local sink | OA-10 |
| 2 | A real Kick test app with webhook delivery through the edge proxy: signature verification with the real key, and a reply through the API | OA-10 |
| 3 | One short broadcast per platform to a private or unlisted test channel: RTMP connects, a BRB cut is visible, then stop | OA-11 |
| 4 | If the owner designates hardware, qualify the `nvidia_xorg` profile (D-07) | owner |

Each item records its date, approver and result in `WALKTHROUGH.md`.

**Release criteria (`RELEASE_CHECKLIST.md`) for `irlosd` 0.1.0-alpha**
1. `live/live.sh all` is green on `ssh GPU`: every S01–S13 check PASSes, except checks tagged pending-hardware or manual.
2. E01–E15 pass in every applicable mode, with evidence in `WALKTHROUGH.md`.
3. The soak meets its thresholds (development-hardware labels), with zero customer impact throughout.
4. The manual checklist is completed, or deferred in writing by the owner.
5. There is no open defect of severity "can stop or corrupt a broadcast" or "cross-seat access".
6. A canary-secret scan of logs, snapshots, images and artifacts is clean.
7. Release packaging is reproducible from a tagged commit. `CHANGELOG.md` documents the limitations:
   - a single destination unless D-04;
   - automatic recovery needs a freshness contract (D-13);
   - unattended dialogs need D-25;
   - capacity is not validated on production hardware (D-08).
8. `seatctl` JSON conforms to the IrlosWeb agent contract, or the divergences are approved (D-06/D-11).
9. Every open Appendix B question that affects production is answered or listed as a release limitation.

#### 7. Implementation tasks

**Unit A — scenario runner and E01–E04 (checkpoint S13-L1)**
1. Write the scenario runner, `faults.sh` and the first scenarios.
2. Prove the runner's own assertions with a deliberately failing scenario.

**Unit B — E05–E15 (checkpoints S13-L2, S13-L3)**

3. Write the remaining scenarios, bare-process first and then in seats.
4. File defects against the owning modules and re-run that step's checks.

**Unit C — soak, manual items and release (checkpoint S13-L4)**

5. Write `soak.sh` and run it in an approved window. Record the results.
6. Prepare the manual checklist and request its approvals.
7. Assemble the release artifacts and the checklist, and present them to the owner. **Do not tag or publish a release without owner approval.**

#### 8. Error handling and recovery

- A failing scenario stops the run, keeps all logs and runs cleanup.
- The runner never retries a failed scenario automatically; flakiness is a defect to investigate.
- A forbidden-event assertion fails immediately.
- A customer-impact violation stops the whole run and is reported to the owner.
- An interrupted soak restarts from the beginning.

#### 9. Concurrency and lifecycle

- Scenarios run sequentially. Multi-seat scenarios use `seatctl` only.
- Every process is tracked by `start_tracked` or belongs to an `irlos-dev-*` or seat container, so `clean` restores a pristine dev root.
- No scenario sends output to a real platform; that happens only in the manual checklist, with OA-11.

#### 10. Security and isolation

- The canary-secret scan covers every artifact, including fetched Studio logs (redacted).
- The isolation probes (S12-L3) are re-run during the soak.
- Real credentials appear only in manual items, in owner-provided secret files, and never in `WALKTHROUGH.md`.

#### 11. Local testing

- Scenario-file schema validation.
- The runner's assertion logic, against recorded snapshot fixtures.
- Fault-script argument validation.
- The full `cargo test --all-targets`, `clippy` and `fmt` gate.
- If the owner approves the tool install: a `cargo deny` or `cargo audit` advisory check, and a license inventory of the image.

#### 12. Live testing on `ssh GPU`

| Check | Proves | Expected |
|---|---|---|
| S13-L1 | E01–E04 (bare and seat) | PASS |
| S13-L2 | E05–E10 | PASS |
| S13-L3 | E11–E15 | PASS |
| S13-L4 (`--pending-hardware` for full capacity) | Soak ≥ 60 min (OA-14) against the thresholds | PASS with development-hardware figures |
| S13-M1…M4 (`--manual`) | Manual platform checklist items | recorded, or deferred by the owner |

Run order: `live/live.sh all` (the full regression), then `live/live.sh check s13`, then `live/soak.sh`. Cleanup: `seatctl destroy` of all dev seats, then `live.sh clean`.

#### 13. Acceptance criteria and definition of done

1. All release criteria in §6 are met.
2. `live/WALKTHROUGH.md` records, for every scenario: date, commits, route, command, result, key log excerpts (redacted) and measurements (development hardware).
3. All S01–S13 checks PASS in the final run (pending-hardware and manual tags as allowed).
4. The owner has reviewed the release checklist. The release is tagged only with the owner's approval.

#### 14. Handoff to later steps

There is no later step. Hand to the owner:
- the release-candidate artifacts;
- `WALKTHROUGH.md`;
- the manual checklist, completed or deferred;
- the capacity figures and the production-hardware validation plan (re-run S12-L4 and the soak on production hardware with 1..8 seats);
- the remaining open questions and limitations.

#### 15. Open questions and assumptions

- Capacity validation on production hardware (D-08).
- Real platform behaviour (Q-08, Q-09).
- The relay's freshness contract (D-13).
- Unattended dialogs (D-25).
- Simulcast scope for the alpha (Q-15, D-04).
- Release packaging format and the GPL source offer (Q-21).
<!-- STEP 13 END -->

---

## Part IV — Appendices

### Appendix A — Mapping from the old 16 steps to the new 13

| Old step (16-step plan) | Content | New step(s) | Notes |
|---|---|---|---|
| 1 Rust workspace and pruning | package, pruning, IDs, scaffolding, config, safe startup, subcommands | **01** | single package kept (no crates workspace) |
| 2 Structured error registry | `IrlosError`, `PublicFailure`, dispositions, certainty | **01** (model), **02** (ICP codes) | |
| 3 ICP framing codec | u32 BE framing 2..16384, strict JSON | **02** | |
| 4 ICP schemas and handshake | hello, ten operations, errors; seven proposed extension operations | **02** (v1.0 core aligned to the plugin), **03** Unit D (extensions, gated) | schemas corrected (D-02) |
| 5 Unix-socket controller and common boundary | `UnixSocketClient`, common types, trait, peer checks, backoff | **03** | timing per ICP §9 (D-03); peer modes defined |
| 6 OBS v5 bridge and dispatch | factory, exact mappings, media-repair mapping | **03**; repair mapping used by **08** | bridge isolated by network namespace (D-22) |
| 7 SLS telemetry | runner, `PollOutcome`, `noalbs_sls_json`, freshness | **07** | never `?reset`; interval counters |
| 8 Belabox adapter | 0/1 semantics, optional modem model | **07** | primary provider for Belabox-cloud routes |
| 9 Pure hysteresis policy | 3 bad / 5 good, providers, holds | **08** | |
| 10 Supervisor, profiles, destinations | process owner, profile generation, launch, shared-encoder fanout | **04**; fanout companion gated (D-04); display/container parts to **11** | generated keys verified against source (D-21); dialog inventory (D-25) |
| 11 State actor and reconciler | actor, commands, outputs, lifecycle, persistence, shutdown; stale-data BRB; media repair | **05** (core), **08** (automation) | split explained in §4; UI-blocked detection added |
| 12 Chat gateway (no prompt existed) | Twitch and Kick commands, permissions, replay | **09** | Kick via signed webhooks (architecture report §4.6) |
| 13 Admin socket (no prompt) | `/run/irlos/irlosd.sock`, JSON-RPC 2.0 | **06** | moved earlier (D-01); module `src/admin/`, not `src/admin.rs` |
| 14 TUI (no prompt) | attach-only `irlosd tui` | **10** | |
| 15 Docker seats and display (no prompt) | Dockerfile, entrypoint, compose, 8 seats | **11** (one seat), **12** (multi-seat, isolation, `seatctl`) | 8 seats only on production hardware; `seatctl` aligned to the IrlosWeb agent |
| 16 End-to-end verification (no prompt) | mock SLS, BRB/LIVE timing, chat stop, walkthrough, capacity | **13**, plus per-step live checks from 01 | |
| — (new) | local + `ssh GPU` development and live-testing workflow | **01**, extended by every step | dev box (Route A) or native (Route B) |

### Appendix B — Open questions

| ID | Question | Why it matters | Used in |
|---|---|---|---|
| Q-01 | `ssh GPU` was unreachable during drafting and revision: five attempts timed out (2026-11-02 and 2026-11-03). Every live-environment fact in §6 is a RECON-based assumption | the workflow, build route and prerequisites depend on it | 01 |
| Q-02 | RECON shows the `GPU` alias reaches the **production customer GPU host**. Is that still true, and may it be used for development? When, how heavily, and with what coordination with the human operator on it? | customer broadcast safety; NVENC and VRAM availability | §9.8, 01, 12 |
| Q-03 | What is the actual concurrent NVENC session limit for the installed driver on this TU117 GTX 1650, measured with `pmon`? (The session counter is unreliable here.) | admission and the seat ceiling | 04, 12 |
| Q-04 | Which tools are installed on `ssh GPU`: compiler, `pkg-config`, OpenSSL headers, CMake version, `ffmpeg` with NVENC, Xvfb, Docker, NVIDIA Container Toolkit, unprivileged user namespaces? How much disk is free in the home directory? | OA-2 to OA-7; Route B namespaces | 01, 03 |
| Q-05 | Can IrlosStudio be built natively on Ubuntu 22.04 with only user-local dependencies, or is the 24.04 dev box required? | D-09 | 01 |
| Q-06 | For the deployed relay: format, version, auth, `?publisher=` response shape, sanitized samples, and the source evidence for a `current_on_request` freshness contract. Is the stats endpoint seat-scoped? | the parser contract; whether automatic recovery to LIVE is possible at all | 07, 08 |
| Q-07 | Which tenants use Belabox cloud rather than the relay? What are Belabox's endpoint format and per-modem fields, and is it reachable from a seat? | provider config; the modem adapter | 07 |
| Q-08 | Kick external facts: webhook signature headers and algorithm, public-key endpoint, `chat.message.sent` payload, chat-send API and OAuth scopes. None of these could be verified here | the Kick adapter | 09 |
| Q-09 | Are Twitch and Kick test accounts or channels available for chat tests that need no broadcast? | the manual checks | 09, 13 |
| Q-10 | Will production run IrlosStudio with the plugin, or stock OBS? RECON saw stock OBS 30.2.3 in production | which driver production uses | 03 |
| Q-11 | Where are production seat images built and stored (registry), and who signs them? | Step 11 packaging | 11 |
| Q-12 | Is the `nvidia_xorg` profile feasible inside a container with the host driver, without host X changes? Does the NVENC soft path (CUDA) work under Xvfb in a container? | GPU compositing; capacity | 04, 11 |
| Q-13 | Do the plugin and its conformance suite pass? No results are recorded; S01-L5/L7 will answer this | ICP readiness | 01, 02 |
| Q-14 | What UID/GID scheme will hosts use for seats and operators (ranges, user namespaces)? | admin principals; isolation | 06, 11, 12 |
| Q-15 | What is the alpha's destination scope: one platform per seat, or Twitch and Kick simulcast (needs D-04)? Kick has no `rtmp-services` entry | output mode | 04, 13 |
| Q-16 | Must the alpha resume broadcasting automatically after a daemon or container restart? The documents say no | the customer's "one button" expectation | 05 |
| Q-17 | Will the IrlosWeb GPU-host agent call `seatctl`, the admin socket, or both? Do `studio_port` and `display_num` become `null` or optional in the frozen contract? | the integration contract with IrlosWeb | 06, 12 |
| Q-18 | Is host clock synchronization reliable on GPU hosts? It matters for Kick webhook timestamp and skew checks | chat security | 09 |
| Q-19 | Which displays, ports, users and processes on `ssh GPU` belong to customers (the preflight lists them read-only)? What are the egress allowlist endpoints per deployment? | R6; the firewall templates | 01, 12 |
| Q-20 | Are sanitized legacy NOALBS configs of production tenants available for `migrate-config` tests and threshold migration? | migration correctness | 01, 08 |
| Q-21 | What is the release packaging format (image only, or also a tarball), and how is GPL source offered for shipped images? | release | 11, 13 |
| Q-22 | Does `-DENABLE_BROWSER=OFF` cleanly disable a populated `plugins/obs-browser` submodule (no CEF), given `plugins/CMakeLists.txt` requires the directory to exist? | the IrlosStudio build | 01 |
| Q-23 | While IrlosStudio shows a modal `QMessageBox`, does the dialog's nested event loop still run queued ICP UI tasks, or does every ICP call time out? Observed in S04-L5 and S05-L4 | the D-25 decision | 04, 05 |

### Appendix C — Decisions needing owner review

| ID | Decision taken in this roadmap | Previous position or conflict | Alternative |
|---|---|---|---|
| D-01 | **Step order changed:** the state owner is Step 05, before telemetry, policy and chat; supervision is 04; admin IPC is 06 (§4). Kept in revision 2 | the old 1–16 order (architecture report §4.1); the drafting task's suggested grouping | keep the historical order and test through fake sinks until the state owner exists |
| D-02 | **The ICP client follows `protocol.md` v1.0 and the plugin as implemented:** `get_current_scene` returns `{scene}`; start/stop return `{previous_state, state}`; the 5-state output enum; the `get_stats` field names; 14 error codes including `obs_timeout`, `output_busy`, `output_start_failed` and `response_too_large`; capability detection via `unknown_operation`; `operation_pending` dropped in favour of `output_busy` | the old Step 4 schema table and old Step 2 code list (orientation conflict C3) | change the plugin to the old Step 4 schema (a major protocol change) |
| D-03 | **ICP timing:** client request timeout ≥ 5000 ms; reconnect 250 ms → 5 s with no jitter; Studio polled at 1 Hz | old Step 5 used 2 s timeouts and 500 ms/1/2/5 s with jitter; old Step 11 polled every 2000 ms (C4) | keep 2 s, which preempts the plugin's 3 s `obs_timeout` |
| D-04 | **IrlosStudio companion package (owner-gated):** ICP v1.1 extensions (`get_capabilities`, media state and repair, named outputs) and the shared-encoder destination manager. Until approved: `native_single` output, and media repair only via `obs_v5` | old Steps 4, 5 and 10 treated these as required companion work | approve the companion now and make simulcast an alpha requirement |
| D-05 | **Seat socket and UID model:** one unprivileged seat UID runs both irlosd and Studio; the client checks peer UID and child PID; `/run/irlos` is a private per-seat `0700` tmpfs | `protocol.md` §2 specifies a `2770 root:irlos` directory and a separate `irlosstudio` user; the architecture report said "service-owned 0660" (C5) | separate UIDs in a shared group (needs privileged process setup) |
| D-06 | **Deployment model: Docker seats managed by `seatctl`**, whose command set and JSON shapes match the frozen IrlosWeb agent contract (generic unit names; `studio_port` and `display_num` returned as `null`) | IrlosWeb `PROVISIONING_SPEC.md` specifies systemd template units with per-tenant users; production today runs root systemd units (C11) | systemd units (the seat init script and `seatctl` vocabulary are reusable) |
| D-07 | **Display:** `software_xvfb` for development and testing; `nvidia_xorg` qualified only on owner-designated hardware, with no silent fallback | old plans used Xvfb `:1..:8` with `OBS_USE_EGL`; production uses the host `:0` with hardware GL and a dummy plug (owner notes, RECON) (C12) | qualify `nvidia_xorg`, or pursue headless EGL Studio (the owner's long-term idea) |
| D-08 | **Capacity:** the 8-seat target is not validated on the GTX 1650; admission uses measured limits (VRAM first, as IrlosWeb expects); production capacity is unknown until measured | the master spec's 8-seat compose; "8 NVENC sessions" claims (C10) | set an interim alpha seat target |
| D-09 | **IrlosStudio build route on `ssh GPU`:** Route A (recommended) is an Ubuntu 24.04 dev box container in which Studio, Xvfb, the fakes and irlosd run (needs Docker, OA-6). Route B is native, with user-local dependencies plus apt packages (OA-3/4/5/7). Both need `OBS_VERSION_OVERRIDE=30.2.3`, `ENABLE_BROWSER=OFF` and populated submodules | not specified anywhere | build elsewhere and ship artifacts or an image |
| D-10 | **Kick ingress:** official signed `chat.message.sent` webhooks through a trusted TLS edge proxy to a per-seat webhook socket (architecture report §4.6, preserved). This needs new public infrastructure | the master spec said "Kick WebSocket"; the legacy Pusher feed lacks stable identities | a Twitch-only alpha, with Kick read-only or disabled |
| D-11 | **No encoder-bitrate control:** LOW changes composition only. `seatctl control set_bitrate` returns an error | the IrlosWeb agent contract lists `set_bitrate` (C19) | add an ICP operation (protocol change) |
| D-12 | **Chat command set:** `start`/`go`, `stop`/`end`, `brb`, `live`, `scene`, `bitrate`, `fix` (documented), plus `auto` (new, to release a hold), plus scene shortcuts migrated from legacy `optionalScenes` (`!privacy`, `!starting`, `!ending`). Other legacy commands survive only as configurable aliases. Replies are English-only (rust-i18n dropped) | the legacy command set and locales in `src/chat/` (C22) | keep more legacy commands, or localization |
| D-13 | **Telemetry trust:** the source-supported `noalbs_sls_json` schema; zero means hold; never `?reset`. `ResponseOnly` freshness cannot recover LIVE, so **automatic recovery, including leaving the startup BRB, needs a provisioned freshness contract**. Daemon-side filtering is not tenant isolation | the old crash course and master spec assumed XML or snake_case; legacy NOALBS recovers on any response (C8) | relax the freshness rule for the alpha (risk of false recovery) |
| D-14 | **Scene names:** template defaults `Live`, `Low`, `BRB`, `Offline`, `Starting`; the switcher maps normal/low/offline to `Live`/`Low`/`BRB` | LIVE/BRB in the master spec; `Live`/`BRB`/`low` plus `STARTING`/`ENDING`/`PRIVACY`/`REFRESH` in production (RECON); the test server's names (C20) | other canonical names, or templates per tenant |
| D-15 | **Workflow:** rsync sync (no Git on `ssh GPU`), dev root `~/irlos-dev`, builds on `ssh GPU`, the `live/` layout, the `LIVE_RUNS.md` run log, display range `:101`–`:199` | not specified | Git-based sync; build in CI |
| D-16 | **Toolchain:** Rust installed user-locally with rustup (or inside the dev box) and pinned with `rust-toolchain.toml` (≥ 1.88); `clap`, `nix`, `hyper` and `sha2` added as needed | not specified | distribution packages; hand-written CLI parsing |
| D-17 | **Dropped legacy features:** the Warp `/ws` API; PubSub raid auto-stop; Twitch transcoding checks; the offline-timeout stream stop; `recordWhileStreaming`; starting-scene automation; username-based admin; the eleven unused telemetry providers. Legacy chat is deleted in Step 05 and rebuilt in 09 | partly in old Steps 1, 9 and 11; raid/PubSub retention was undecided | migrate selected features as intents |
| D-18 | **Plugin stale-socket behaviour:** the plugin unlinks any existing socket before binding without checking for a listener (it does remove only its own inode at exit). The roadmap relies on `RuntimeLock` and per-seat private directories instead of a plugin fix | `protocol.md` §2 says "MUST unlink"; the plugin README differs (C6, C32) | probe before unlinking, in the companion (OA-9) |
| D-19 | **Studio stop uses SIGINT, then SIGKILL**, with `ConfirmOnExit=false` and outputs stopped first. IrlosStudio has no SIGTERM handler | old Steps 10 and 11 assumed SIGTERM (C7) | add a SIGTERM handler in the companion (OA-9) |
| D-20 | **TUI packaging:** the `irlosd tui` subcommand (architecture report §4.7) | the onboarding PDF mentions a separate `irlos-tui` (C29) | a separate binary |
| D-21 | **Unattended-launch corrections:** `--disable-missing-files-check`; `user.ini` `FirstRun=true`, `ConfirmOnExit=false`, `MaxLogs=10` and `RecordWhenStreaming=false`; `global.ini` `Pre31Migrated=true`; bounded `[Output] Reconnect/RetryDelay/MaxRetries` | the old Step 10 flag list lacked these; a first-run wizard would block ICP | — |
| D-22 | **OBS v5 fallback isolation:** the client connects only to loopback, and the obs-websocket server runs **only inside a private network namespace** (seat container or dev box), with auth required. Revision 1 said "loopback only", which obs-websocket cannot enforce | production obs-websocket today is bound publicly with a shared password, on a host with no firewall (RECON; C15, C34) | firewall rules per host (needs OA-13) |
| D-23 | **Target binary:** IrlosStudio from `irlos/icp-plugin`; stock OBS only through the OBS v5 bridge | production appears to run stock OBS (C14) | — |
| D-24 | **No automatic broadcast resume** after a daemon or container cold start (old Step 11, preserved); an optional same-epoch resume after a Studio crash | customer expectations (Q-16) | durable resume (needs a new safety design) |
| D-25 | **Unattended error dialogs.** IrlosStudio shows modal `QMessageBox`es on stream-start errors (always), stream encode errors (always), connection failures (whenever the window is visible) and invalid recording paths (`window-basic-main.cpp:6248-6256`, `:6732-6748`). These block the UI thread and therefore ICP. The roadmap detects them (`engine.studio_ui_blocked`, `Degraded`) and recovers only through an authorized lifecycle cycle; the opt-in `engine.restartStudioWhenUiBlocked` defaults to `false` | no previous document handled this (the orientation noted it as a risk) | **recommended:** an IrlosStudio unattended mode that logs instead of showing dialogs (companion, OA-9); or enable the automatic cycle by default |

### Appendix D — Source documents

All of these except the revision task itself were read for revision 1, and all were re-checked for revision 2 (2026-11-03). None of the revision 1 sources changed in between: same commits, and the documents' mtimes are older than revision 1.

| Document | Role | Revision 2 re-read |
|---|---|---|
| `Downloads/irlosd_13_step_roadmap_task.md` | the drafting task (the only copy found) | in full |
| `Downloads/irlosd_roadmap_resume_task.md` | the revision task | in full |
| `Downloads/irlos_orientation_prompt.md` and the orientation report (earlier in this session) | the orientation method and findings | in full |
| `irlosd/prompts/irlosd_alpha_master_specification.md` | the 16-step sequence (superseded) | in full |
| `irlosd/prompts/architecture_verification_steps_01_11.md` | the integration contract and A–E gates for old Steps 1–11 (superseded for sequencing; contracts carried over) | in full |
| `irlosd/prompts/step_1_…` to `step_11_…` | old step prompts (superseded) | in full |
| `irlosd/prompts/development_specification_audit.md` | the earlier audit (already superseded) | in full |
| `irlosd/CRASH_COURSE(3).pdf`, `Downloads/ONBOARDING.pdf` | onboarding and company background (financials not reproduced) | irlosd/ICP sections and the onboarding rules |
| `IrlosDSteps`, `steps to create irlosd`, `IrlosDPlanning{1,2,3chatgpt}.txt` | owner directives and planning sessions (Rust, NOALBS fork, Unix socket, Docker, SRT stats, belabox.net, TUI, 2–3 page prompts) | owner notes in full; owner decision points in the planning log |
| `.gemini/…/plan_irlosd_daemon.md` | the earlier plan and verification plan | in full |
| `IrlosStudio/plugins/irlos-control/{protocol.md, src/plugin/plugin-main.cpp, src/icp/{session,server}.cpp, src/plugin/obs_backend.cpp, src/plugin/ui_task.*}` at `564647bcb` | the ICP v1.0 contract and server implementation | `protocol.md` in full; the listed sources in full or in the relevant parts |
| `IrlosStudio/UI/{obs-app.cpp, window-basic-main.cpp, window-basic-main-outputs.cpp, window-basic-main.hpp}`, `plugins/obs-nvenc/nvenc.c`, `plugins/CMakeLists.txt` | launch flags, signals, config files and keys, dialogs, encoder selection and reroute | the cited lines |
| `IrlosStudio/{PHASE_4_WEBSOCKET_VERIFY.md, PHASE_6_REPORT.md, IRLOS_STRIP_PLAN.md}` | historical build and test reports, strip decisions | Phase 4/6 in full; strip-plan decisions |
| `irlosd/{Cargo.toml, src/**, README.md}` at `dab2665` | current code (NOALBS 2.21.1) | manifest, `lib.rs`, chat imports, config structs, cited lines |
| `IrlosWeb/tasks/{RECON, PROVISIONING_SPEC, CONTRACT_AGENT_API, README, TASK_0_RECON, TASK_1_GPU_AGENT, TASK_3_INTEGRATION}.md` | the production snapshot (2026-07-30), the hosting plan, the frozen agent contract | in full (host details not reproduced) |
| Not re-read: `IrlosWeb/tasks/TASK_2/4/5`, the superseded Gemini step drafts, `cockandballs/` and `irlosd_prompts.tar` (byte-identical copies), dashboard planners and AI conversation logs | web app, Stripe and site work outside irlosd; superseded or duplicate material; excluded by the owner or because of secret risk | — |

### Appendix E — Glossary

| Term | Meaning |
|---|---|
| **Actor / state owner** | The single task that owns `SessionState` and is the only writer of session truth (Step 05) |
| **Admission generation** | A counter that invalidates queued Starts once a Stop is accepted |
| **BRB** | "Be right back": the scene shown while the feed is bad; outputs keep running |
| **Capability (command)** | A permission such as `Broadcast` or `Scene` held by a `Principal` |
| **Capability (controller)** | What the connected Studio supports, discovered per connection |
| **Customer impact** | The before/after comparison of customer GPU processes and VRAM around every GPU-using live check (§9.8) |
| **Development hardware** | `ssh GPU` (GTX 1650). Every measurement taken there carries this label |
| **Dev box** | The Route A container `irlos-dev-box` (Ubuntu 24.04) in which Studio, Xvfb, the fakes and irlosd run for live checks |
| **Epoch** | A random identifier for one `serve` run. Commands from an old epoch are rejected |
| **Freshness evidence** | How sure we are that a telemetry sample is current: `ResponseOnly`, `ValidatedSensorTime` or `CurrentOnRequestContract` |
| **Generation** | A counter for a process, connection, config or provider instance. Stale facts from older generations are discarded |
| **Hysteresis** | Requiring several consecutive samples (3 to degrade, 5 to recover) before switching |
| **ICP** | IRLOS Control Protocol: length-prefixed JSON over a Unix socket, served by the `irlos-control` plugin inside IrlosStudio |
| **IrlosStudio** | The IRLOS fork of OBS Studio 30.2.3 |
| **Live check** | A registered check run by `live/live.sh` on `ssh GPU`. IDs are `SNN-LK`; manual items are `SNN-MK` |
| **NOALBS** | The open-source low-bitrate switcher irlosd is derived from |
| **NVENC** | NVIDIA's hardware video encoder. GeForce drivers limit concurrent sessions |
| **OA-NN** | An owner-approved action (§9.9) |
| **Receipt** | The state of a submitted command: `Accepted`, `Applied`, `CompletedUnverified`, `OutcomeUnknown`, `Failed` or `Superseded` |
| **Seat** | One customer session: one container with its display, irlosd and IrlosStudio |
| **SentinelHold** | A telemetry value that means "no information" (SLS 0, Belabox 1): neither good nor bad |
| **sls** | SRT Live Server, the ingest relay. Its HTTP stats are the primary telemetry for relay tenants |
| **`SessionHandle`** | The only client API to the actor |
| **Simulcast / fanout** | One encoder group feeding several RTMP destinations (gated, D-04) |
| **SRT / RTMP** | The contribution protocol from the field / the delivery protocol to platforms |
| **UI blocked** | IrlosStudio's UI thread held by a modal dialog, so every ICP call times out (D-25) |

