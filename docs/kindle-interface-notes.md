# Kindle Interface Notes

This document records what we currently know about the Kindle display interface used by
`kindle-client`, what was confirmed on-device, what remains unknown, and which probing steps
should be treated as risky.

Unless otherwise stated, the findings here are for:

- Hardware: Kindle Paperwhite 3
- Firmware: `5.16.2.1.1`

## Confirmed Device Facts

- Device kernel reported over SSH:
  - `Linux kindle 3.0.35-lab126 #8 PREEMPT Tue Aug 1 12:49:59 UTC 2023 armv7l GNU/Linux`
- Framebuffer device:
  - `/dev/fb0`
- EPDC sysfs path:
  - `/sys/devices/platform/imx_epdc_fb`
- Framebuffer class name:
  - `/sys/class/graphics/fb0/name` -> `mxc_epdc_fb`
- Visible framebuffer mode:
  - `1072x1448`
- Framebuffer format:
  - `8` bits per pixel
- Reported virtual size:
  - `1088,6144`
- Loaded kernel modules include:
  - `mxc_epdc_fb`
  - `mxc_epdc_eink`
- Device serial / board identifiers observed on 2026-03-26:
  - `/proc/usid` -> `G090G105535400SS`
  - `/proc/board_id` -> `06702091534709U4`
- Confirmed hardware identity:
  - Kindle Paperwhite 3 on the Wario platform.
- Basis for hardware identification:
  - `Hardware` reports `Freescale i.MX 6SoloLite based Wario Board`
  - the serial code `0G1` maps to `KindlePaperWhite3WiFi` in KindleTool / KOReader device tables

## Newly Confirmed Since The Earlier Notes

These findings were gathered after the earlier screen-ready investigation and materially narrow the
problem.

### 0. The Current Rust `ioctl` FFI Was Wrong For 32-bit ARM

The Kindle userspace here is `armv7l`, so `ioctl` request values are passed as C `unsigned long`
(32-bit), not `u64`.

The Rust Kindle code was declaring:

- `ioctl(fd, request: u64, ...)`

That is ABI-wrong on this device family.

Why this matters:

- A standalone probe binary was changed to use the correct request type (`c_ulong`) while keeping
  the same candidate request numbers and update structs.
- After that change:
  - the generic non-Kindle update structs still returned immediate `EINVAL`
  - but the PW3/Wario Kindle request `0x4048462e` no longer returned immediate `EINVAL`
  - instead, probing that path led into a much more dangerous state and the device rebooted during
    testing

Practical implication:

- Earlier ioctl failures are not trustworthy until the main app code uses the correct request type.
- The request-width bug and the missing PW3 Kindle struct layout were both real issues.
- Future ioctl probing should now be much more conservative, because the correct request width can
  reach real driver code instead of being rejected cheaply.

### 1. The Likely `MXCFB_SEND_UPDATE` ABI For This Kindle

The current app does not probe the exact struct layout used by the PW2/PW3/Voyage Kindle driver
family.

Strong evidence from Kindle community headers used by KOReader / FBInk:

- `MXCFB_SEND_UPDATE` on K5.1-style Kindle kernels is:
  - `_IOW('F', 0x2E, struct mxcfb_update_data)`
  - request value `0x4048462e`
- That Kindle struct is not the plain upstream Freescale layout.
- It includes two Lab126 fields inserted before `temp` and `flags`:
  - `hist_bw_waveform_mode`
  - `hist_gray_waveform_mode`
- Kindle PW2/PW3/Voyage layout:
  - `rect`
  - `waveform_mode`
  - `update_mode`
  - `update_marker`
  - `hist_bw_waveform_mode`
  - `hist_gray_waveform_mode`
  - `temp`
  - `flags`
  - `alt_buffer_data`

Important consequence:

- The current app probes these variants:
  - `use_alt_buffer`-style
  - `flags` without hist fields
  - `flags + virt_addr`
  - `flags + virt_addr + dither`
- None of those match the Kindle PW3 layout above.
- This is now the most likely reason `MXCFB_SEND_UPDATE` keeps failing while other ioctls work.

### 2. The Wait API Is Richer Than We First Confirmed

For this Kindle family, there are two relevant marker waits:

- `MXCFB_WAIT_FOR_UPDATE_COMPLETE`
  - request value `0xc008462f`
  - argument type: `struct mxcfb_update_marker_data`
  - fields:
    - `update_marker`
    - `collision_test`
- `MXCFB_WAIT_FOR_UPDATE_SUBMISSION`
  - request value `0x40044637`
  - argument type: `u32`

On-device evidence:

- `mxc_epdc_fb.ko` exports symbols for:
  - `mxc_epdc_fb_wait_update_complete`
  - `mxc_epdc_fb_wait_update_submission`
- module strings include:
  - `Timed out waiting for update completion. Marker : %d`
  - `Timed out waiting for update submission. Marker : %d`

Practical implication:

- There is probably a kernel-supported distinction between:
  - "the driver accepted / submitted the update"
  - "the panel finished drawing it"
- That is the best lead so far for answering both:
  - "can I safely queue another draw?"
  - "is the panel still visibly busy?"

### 3. The Driver Does Not Appear To Export A Passive Queue/Busy File

Read-only inspection on 2026-03-26 found:

- EPDC sysfs:
  - `/sys/devices/platform/imx_epdc_fb/...`
- `debugfs`:
  - no obvious EPDC queue-state nodes
- procfs:
  - `/proc/eink/...` exists, but appears focused on panel / waveform metadata
  - small readable files include:
    - waveform version / checksum / source
    - panel id / bcd
  - no obvious queue depth or "busy" boolean was found

Meanwhile, the reference EPDC driver and Kindle module strings clearly point to internal queue data:

- pending-update lists
- marker lists
- merge logic
- queue mutex

Working conclusion:

- There is no confirmed passive readable queue-depth interface.
- The intended synchronization path is almost certainly marker-based waits, not polling a sysfs
  busy bit.

### 4. Waveform / Draw-Mode Information Is Better Than We Thought

Observed on-device:

- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_waveform_modes` prints:
  - `mode_version:0x19`
  - named mappings for:
    - `init`
    - `du`
    - `du4`
    - `gc16f`
    - `gc16`
    - `gc4`
    - `gl4`
    - `gl16inv`
    - `gl16f`
    - `gl16`
    - `reagld`
    - `reagl`
- `/proc/eink/waveform/info` reports waveform metadata, including:
  - `mode version: 0x19`
  - `frame rate: 0x85`
  - `bit depth: 0x05`

Kindle user-space headers for the Wario / PW3 family expose these update-mode constants:

- `INIT = 0x0`
- `DU = 0x1`
- `GC16 = 0x2`
- `GC16_FAST = 0x3`
- `A2 = 0x4`
- `GL16 = 0x5`
- `GL16_FAST = 0x6`
- `DU4 = 0x7`
- `REAGL = 0x8`
- `REAGLD = 0x9`
- `GL4 = 0xA`
- `GL16_INV = 0xB`
- `AUTO = 257`

KOReader / FBInk Kindle heuristics are informative here:

- On REAGL-capable Kindles, partial content refreshes default to `REAGL`.
- "Fast" feedback still uses `DU`.
- UI-style updates use `GC16_FAST`.
- Full refreshes use `GC16`.
- `A2` is treated as available but undesirable on newer REAGL devices.

That does not prove these are the optimal choices for this game, but it strongly suggests:

- if we want fast game motion, the first serious candidates are `DU` and possibly `DU4`
- if we want lower ghosting for normal partial content, `REAGL` is likely the right higher-quality
  partial mode
- `GC16` / `GC16_FAST` are likely too slow for frame-by-frame game motion, but useful for menus /
  cleanup / full redraws

#### 4a. Extra EPDC State Surfaces

Additional read-only EPDC state observed on 2026-03-26:

- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_temperature` -> `4097` (`0x1001`)
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_reagl` -> `0`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_debug` -> `0`

Important temperature clue:

- `4097` matches the Kindle/PW2-era `TEMP_USE_AUTO` value (`0x1001`), not `TEMP_USE_AMBIENT`
  (`0x1000`).
- FBInk also uses `TEMP_USE_AUTO` on this Kindle family.
- That makes `temp = 0x1001` a more plausible future ioctl candidate than `0x1000`.

#### 4b. Safe Sysfs Draw-Mode Probing Works

The app's existing sysfs draw path is much safer to explore than the ioctl path.

Single-shot tiny region test on 2026-03-26:

- Framebuffer write:
  - `8x1` pixels
  - top-left region
- EPDC command format:
  - `<waveform> <update_mode> <top> <left> <width> <height>`
- These all returned cleanly via `/sys/devices/platform/imx_epdc_fb/mxc_epdc_update`:
  - `AUTO` partial: `0 0 0 0 8 1`
  - `DU` partial: `1 0 0 0 8 1`
  - `DU4` partial: `7 0 0 0 8 1`
  - `GL16_FAST` partial: `3 0 0 0 8 1`
  - `REAGL` partial: `4 0 0 0 8 1`
  - `REAGLD` partial: `5 0 0 0 8 1`
  - `GC16` full: `2 1 0 0 8 1`

Host-observed coarse timings:

- Each single-shot sysfs update returned in about `0.17s` to `0.19s` including SSH overhead.
- That suggests the sysfs write itself returns quickly for all of these modes.

Practical implication:

- On the safe sysfs path, the main gameplay candidates are now clearly:
  - `DU`
  - `DU4`
  - `REAGL`
  - possibly `REAGLD`
- `GL16_FAST` and `GC16` are accepted too, so they remain viable for higher-quality redraws.

#### 4c. `mxc_epdc_update` Is Not A Completion Fence

One conservative queue test was run on 2026-03-26:

- two tiny `DU` partial updates
- back-to-back
- different rows (`top=0` then `top=1`)
- same SSH session

Observed result:

- the two-update command returned in about `0.18s`
- essentially the same coarse timing as a single update

Working conclusion:

- Writing to `mxc_epdc_update` does not wait for visible completion.
- It appears to return once the driver has accepted / queued the work.
- So this path is useful for safe draw-mode exploration, but it does not answer "is the panel done
  drawing?"

#### 4d. Visual Animation Findings From Real Game-Like Tests

Additional on-device visual tests were run on 2026-03-27.

Useful conclusions:

- Large disappearing path animation should be treated as ruled out for now.
  - Across `DU`, `DU4`, `REAGL`, and `GL16_FAST`, the disappearing region always produced an
    intrusive dark refresh transient.

- Player blink is plausible.
  - The tiny eye-only dirty rect was acceptable.
  - Across `DU`, `DU4`, `REAGL`, and `GL16_FAST`, no clear winner was visible by eye.

- Box vanish is good enough to implement later.
  - The acceptable version used:
    - the real background image
    - a shrinking box over 10 phases
    - about `150ms` between phases
    - restoration of only the newly disappeared strips, not the full box region
  - Across `DU4`, `DU`, `REAGL`, and `GL16_FAST`, the effect looked indistinguishable by eye.
  - Preserve the animation structure, not a specific waveform choice.

Important caution:

- Video captures showed that smaller or upper regions can visibly settle before a larger changing
  region does.
- Queued updates can therefore overlap visibly.
- Host-side sleeps are only approximate pacing tools, not proof that the panel has finished
  drawing.

### 5. Sleep Control Likely Goes Through `powerd`, Not `Lab126UI`

Current device state observed on 2026-03-26:

- `powerd` is running
- `lipc-daemon` is running
- `com.lab126.powerd` is published on LIPC
- `/sys/power/state` reports:
  - `standby mem`
- `powerd` upstart job starts on:
  - `started lab126`
  - not `started lab126_gui`

Published `com.lab126.powerd` properties include writable controls:

- `powerButton`
- `wakeUp`
- `preventScreenSaver`
- `touchScreenSaverTimeout`
- `deferSuspend`
- `abortSuspend`
- `rtcWakeup`

Additional evidence:

- KOReader uses:
  - `lipc-set-prop -i com.lab126.powerd powerButton 1`
  - as its suspend toggle path on Kindle
- KOReader explicitly notes there is no distinction between:
  - a real power-button press
  - `powerd_test -p`
  - `lipc-set-prop -i com.lab126.powerd powerButton 1`
- `powerd` binary strings mention:
  - `/sys/power/state`
  - `screenSaver`
  - `readyToSuspend`
  - `wakeupFromSuspend`
  - `preventScreenSaver`
  - `wakeUp`

Known negative result from actual game/runtime testing:

- In the stripped-down runtime used for the game, explicitly triggering:
  - `lipc-set-prop -i com.lab126.powerd powerButton 1`
  - did nothing.
- This was already tested before wiring the physical power button to an in-app action.

Working conclusion:

- There is still a strong chance we can trigger sleep without the stock GUI by talking to `powerd`,
  but the obvious `powerButton` property is not sufficient in the game runtime.
- A lower-level fallback may exist via:
  - `echo mem > /sys/power/state`
- But that lower-level path has not been tested and may bypass Kindle-specific suspend choreography.
- We have not yet confirmed behavior in the exact runtime state where `lab126_gui` / `Lab126UI`
  has been killed for the game.

Additional observed behavior from real game runtime:

- After `Lab126UI` was killed for the game, the power button no longer put the device to sleep.
- However, the device still auto-sleeps on its own after an idle timeout of roughly 10 minutes and
  shows its normal sleep/screensaver behavior.

Implication:

- Sleep/screensaver capability itself does not depend on `Lab126UI`.
- The missing piece is more likely the explicit power-button-triggered suspend path, input routing,
  or a user-presence / activity signal normally connected to the button handling path.
- That makes it more likely that a `powerd`-level trigger can still work even in the game runtime.

### 5a. Confirmed `powerd` Idle Timer Values

Read via `kdb` on 2026-03-26:

- `system/daemon/powerd/t1_timeout` -> `600`
- `system/daemon/powerd/t2_timeout` -> `60`
- `system/daemon/powerd/r2s_grace_timeout` -> `5`
- `system/daemon/powerd/send_t1_reset_interval` -> `15`
- `system/daemon/powerd/no_suspend` -> `0`

Interpretation:

- The observed "auto sleep after roughly 10 minutes" matches `t1_timeout = 600`.
- `powerd` is therefore definitely still running its idle suspend state machine in the stripped-down
  runtime.

### 5b. Confirmed Raw Power-Button Source Used By `powerd`

Read via `kdb` and procfs on 2026-03-26:

- `system/daemon/powerd/POWERBUTTON` -> `max77696-onkey.0`
- `system/daemon/powerd/SUBSYSTEM_INPUT` -> `input`
- `/proc/bus/input/devices` shows:
  - `Name="max77696-onkey"`
  - `Handlers=kbd event0`
- `/dev/input/event0` corresponds to:
  - `max77696-onkey`

Implication:

- `powerd` appears to listen to the raw PMIC on-key input device directly.
- That suggests stopping `lab126_gui` should not, by itself, remove the hardware source that
  `powerd` uses for the power button.
- So if the button stops causing sleep in the game runtime, the more likely causes are:
  - the button event path is being repurposed in-app,
  - a powerd state/policy difference in that runtime,
  - or a missing follow-up trigger normally issued by higher-level software.

### 5c. Repo-Side Button Handling Is Already In The App

Current app code on the Kindle side:

- [app_driver.rs](/Users/jt/code/sokobanitron/kindle-client/src/app_driver.rs)
- [platform.rs](/Users/jt/code/sokobanitron/kindle-client/src/platform.rs)

Observed behavior in code:

- The app opens `/dev/input/event0` as `POWER_DEVICE`.
- It interprets:
  - short press -> force a full refresh / redraw
  - long press -> restart `lab126_gui`
- The short-press refresh behavior exists because the more natural software suspend path tried so far
  (`com.lab126.powerd powerButton 1`) was a no-op in the game runtime.
- The code does not currently attempt to ask `powerd` to suspend.

Important nuance:

- The app does not appear to `EVIOCGRAB` the power device, so this does not by itself prove that it
  suppresses `powerd`.
- But it does mean there is already a clear application-level place where the button can be rewired
  to a software suspend path once that path is confirmed.

### 5d. Observed Idle Sleep Sequence In The Game Runtime

Observed via the remote watcher helper on 2026-03-26:

- Observed `powerd` event sequence:
  - `15:23:43.946795 goingToScreenSaver 3`
  - `15:25:06.281242 t1TimerReset`
  - `15:25:06.317999 outOfScreenSaver 1`
  - `15:25:06.369179 exitingScreenSaver`
- Observed `powerd.state` transition:
  - `active` through `15:23:43`
  - `screenSaver` starting at `15:23:44`
  - back to `active` at `15:25:06`
  - the timer resets to roughly `600` seconds on wake
- Power state during the entire capture:
  - `Charging: Yes`
  - battery rose from `99%` to `100%`
- Observed `com.lab126.blanket` events:
  - none during this capture

Important interpretation:

- This confirms the stripped-down game runtime still follows a real `powerd`-managed idle path into
  `screenSaver` and back out again on wake.
- However, this specific capture does not show evidence of a deeper kernel suspend that stops
  userspace:
  - the one-second poller kept running continuously
  - SSH remained available
- Because the device was charging during the capture, this may be a charging-only screensaver path
  rather than the deepest battery-powered suspend behavior.
- So for now, the safest conclusion is:
  - we have confirmed the working idle screensaver/state-machine path
  - we have not yet confirmed whether that same path later escalates into a deeper suspend state in
    this runtime

Second capture, also on 2026-03-26, while unplugged:

- Observed unplugged `powerd` event sequence:
  - `15:29:53.721187 notCharging`
  - `15:35:06.303639 goingToScreenSaver 3`
  - `15:35:25.794865 t1TimerReset`
  - `15:35:25.864107 outOfScreenSaver 1`
  - `15:35:25.906740 exitingScreenSaver`
- Observed unplugged `powerd.state` transition:
  - `active` through `15:35:05` with `Charging: No`
  - `screenSaver` at `15:35:06`
  - `Remaining time in this state: 59.506657` immediately after entering `screenSaver`
  - back to `active` at `15:35:25`
  - the timer resets to roughly `600` seconds on wake
- Observed `com.lab126.blanket` events:
  - none during this capture

Interpretation from the unplugged run:

- The behavior matched the charging run in the important ways:
  - idle still enters `screenSaver`
  - wake returns to `active`
  - the userspace poller kept running
  - SSH stayed available
- So even on battery power, this runtime still looked like a `powerd`-managed screensaver state
  rather than a deeper suspend that stops userspace.
- For the purposes of this project, that is still useful:
  - it confirms a working software-controlled "sleep enough" target state already exists in the
    runtime

### 5e. Direct Screensaver Trigger Probes So Far

Tested on 2026-03-26:

- Public references found:
  - KOReader uses `touchScreenSaverTimeout 1` only to reset the normal idle timer, not to force
    immediate sleep.
  - Public Kindle integration scripts use:
    - `lipc-set-prop com.lab126.blanket unload screensaver`
    - `lipc-set-prop com.lab126.blanket load screensaver`
    - as screensaver module management while stopping/restarting the framework.
  - `powerd_test -p` is documented in its own binary strings as:
    - `Simulate power button pressed event`
    - implemented via:
      - `lipc-send-event com.lab126.powerd.debug dbg_power_button_pressed`

Read-only device evidence:

- `powerd` references:
  - `system/daemon/powerd/BLANKET_NAME`
  - `system/daemon/powerd/BLANKET_LOAD`
  - `com.lab126.screensaver`
- `kdb` confirms:
  - `system/daemon/powerd/BLANKET_NAME` -> `com.lab126.blanket`
  - `system/daemon/powerd/BLANKET_LOAD` -> `load`

Controlled direct-trigger tests:

- `lipc-set-prop com.lab126.blanket load screensaver`
  - did not move `powerd.state` out of `active`
  - did not make `com.lab126.winmgr isScreenSaverLayerWindowActive` become `1`
  - the follow-up unload emitted `exitingScreenSaver` in `powerd` and
    `moduleUnloaded "screensaver"` in `blanket`
  - best current interpretation:
    - this manages the screensaver blanket module
    - it does not directly reproduce the working idle `goingToScreenSaver` transition in this
      runtime

- `powerd_test -p`
  - printed `Sending button pressed event`
  - did not move `powerd.state` out of `active`
  - did not make `com.lab126.winmgr isScreenSaverLayerWindowActive` become `1`
  - did not produce any new useful `powerd` event in the observer logs
  - this is consistent with the already-known result that:
    - `lipc-set-prop -i com.lab126.powerd powerButton 1`
    - is a no-op in the game runtime

Current conclusion:

- The working target state is clearly the `powerd` idle `screenSaver` path.
- The obvious "pretend the power button was pressed" entry points are not enough in this runtime:
  - physical button
  - `powerButton 1`
  - `powerd_test -p`
- `com.lab126.blanket load screensaver` is also not a complete substitute for that path.
- The remaining likely avenues are:
  - find a more direct `powerd` trigger for the same idle transition
  - or intentionally force the idle state machine to expire immediately rather than emulating a
    button press

### 5f. Working Manual Entry: Simulated Magnetic Cover Close/Open

Tested on 2026-03-26:

- Sent manually:
  - `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_closed`
  - followed later by:
    - `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_opened`
- `powerd` binary strings show both debug events exist:
  - `dbg_mag_sensor_closed`
  - `dbg_mag_sensor_opened`

Observed result:

- `powerd` events:
  - `15:44:52.907848 goingToScreenSaver 4`
  - `15:45:02.291434 t1TimerReset`
  - `15:45:02.312112 outOfScreenSaver 6`
- `powerd.state` transition:
  - `active` at `15:44:52`
  - `screenSaver` at `15:44:53`
  - back to `active` after the simulated open event
- `com.lab126.winmgr isScreenSaverLayerWindowActive` remained `0` during the test.
- `com.lab126.blanket` emitted no event during the test.
- User-visible behavior during a direct SSH test:
  - the displayed puzzle framebuffer remained visible
  - taps stopped working while `powerd.state` was `screenSaver`
  - after `dbg_mag_sensor_opened`, taps worked again
  - no Amazon-visible screensaver layer was shown on top

Interpretation:

- This is the first confirmed manual trigger that reproduces the same class of useful "sleep
  enough" state in the stripped-down runtime.
- It does not appear to depend on the broken power-button path.
- It is likely emulating the hall-effect / magnetic-cover path rather than the power-button path.
- For project purposes, this is currently the strongest candidate for a software sleep trigger.

Practical implication:

- If the goal is to rewire the physical power button to "sleep enough", the simplest near-term path
  may be:
  - short press -> `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_closed`
  - wake action -> `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_opened`

Follow-up wake test on 2026-03-26:

- After entering this hall-sensor-induced `screenSaver` state, a physical power-button press woke
  the device successfully.
- On wake:
  - the device returned to `active`
  - the app became responsive to taps again
- So the stripped-down runtime still preserves a usable physical wake path for this state.

Important limitation discovered during an earlier app-side experiment:

- Entering `screenSaver` this way is not the same as solving the full button-toggle UX.
- A temporary app change that mapped the wired power button to this hall-sensor path produced an
  apparent "freeze":
  - the app process was still alive and blocked in its normal input poll
  - `powerd.state` was `screenSaver`
  - manual `dbg_mag_sensor_opened` immediately returned the device to `active`
- The actual issue in that experiment was not lack of wake support; it was app-side handling of the
  transition.
- This should now be treated as:
  - a confirmed shell/manual sleep trigger
  - a confirmed physical-button wakeable state in the stripped-down runtime

### 5g. Reliable No-Screensaver Sleep After Reboot

Tested live over SSH after a later reboot on 2026-03-26:

- Direct hall-sensor sleep by itself:
  - `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_closed`
  - now showed the normal Amazon screensaver again.
- Live state while that visible screensaver was active:
  - `powerd.state = screenSaver`
  - `com.lab126.winmgr isScreenSaverLayerWindowActive = 1`
  - `com.lab126.blanket load = screensaver langpicker blankwindow usb`
  - `blanket` process was running

That explains why the old "sleep while leaving the game image visible" behavior disappeared after the
reboot: the `blanket` screensaver module was active again.

Controlled recovery test:

1. `lipc-set-prop com.lab126.blanket unload screensaver`
2. `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_closed`

Observed result:

- `powerd.state = screenSaver`
- `com.lab126.winmgr isScreenSaverLayerWindowActive = 0`
- `com.lab126.blanket load = langpicker blankwindow usb`
- user-visible result:
  - the game image stayed visible
  - the device entered the desired non-interactive sleep state

Important nuance:

- `lipc-set-prop com.lab126.blanket unload screensaver` by itself is not enough.
- If run while already sleeping, it removes the visible screensaver layer but also returns
  `powerd.state` to `active`.
- The working recipe is specifically:
  - unload `blanket`'s `screensaver` module first
  - then enter `powerd` screensaver via `dbg_mag_sensor_closed`

Current best implementation rule:

- Sleep:
  - `lipc-set-prop com.lab126.blanket unload screensaver`
  - `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_closed`
- Wake:
  - physical power button, or
  - `lipc-send-event com.lab126.powerd.debug dbg_mag_sensor_opened`
- If returning control to stock Lab126 UI later, reload the module before restarting the UI:
  - `lipc-set-prop com.lab126.blanket load screensaver`

### 6. `powerd_test` Should Be Treated As Actionful

One surprise from this round:

- Running `powerd_test -h` did not print usage.
- It emitted:
  - `Sending power button held`

Even though that did not visibly break the session, it means:

- `powerd_test` should not be treated as a harmless help/introspection command.
- Future power-management probes should prefer:
  - `lipc-probe`
  - `lipc-get-prop`
  - binary `strings`
  - or a deliberate one-shot `lipc-set-prop` test

## Confirmed EPDC Files

Observed on the device:

- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_debug`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_powerup`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_pwrdown`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_reagl`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_regs`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_temperature`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_update`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_voltcontrol`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_waveform_modes`
- `/sys/devices/platform/imx_epdc_fb/mxc_epdc_wvaddr`

Observed values during the last inspection:

- `mxc_epdc_debug`: `0`
- `mxc_epdc_powerup`: `0`
- `mxc_epdc_pwrdown`: initially `0`, later `1000`
- `mxc_epdc_reagl`: `0`
- `mxc_epdc_temperature`: `4097`
- `mxc_epdc_update`: `1`
- `power/runtime_status`: `unsupported`

## What The App Uses Today

The Kindle client writes the entire framebuffer to `/dev/fb0` and then requests an EPDC update.

Relevant code:

- [platform.rs](/Users/jt/code/sokobanitron/kindle-client/src/platform.rs)
- [display.rs](/Users/jt/code/sokobanitron/kindle-client/src/display.rs)

Current behavior in the platform layer:

- It tries to configure:
  - `MXCFB_SET_AUTO_UPDATE_MODE`
  - `MXCFB_SET_UPDATE_SCHEME`
- It probes several `MXCFB_SEND_UPDATE` ABI layouts.
- If ioctl-based update submission fails, it falls back to writing commands to:
  - `/sys/devices/platform/imx_epdc_fb/mxc_epdc_update`

Important consequence:

- The current app path does not have a confirmed completion fence for display updates.
- The app can submit work, but it does not currently know when the panel is finished drawing.

## What Was Confirmed About IOCTL Support

Confirmed working on this device:

- `MXCFB_SET_AUTO_UPDATE_MODE`
- `MXCFB_SET_UPDATE_SCHEME`
- `MXCFB_GET_PWRDOWN_DELAY`
- `MXCFB_SET_PWRDOWN_DELAY`

Confirmed not working with the struct variants we tested:

- `MXCFB_SEND_UPDATE`

Observed behavior:

- `MXCFB_WAIT_FOR_UPDATE_COMPLETE` did not return `ENOTTY`; it returned `EINVAL` for the test
  markers we passed.
- That suggests the wait ioctl family likely exists on this kernel, but we do not yet have a
  confirmed valid marker-producing update submission path for this specific device.

## What We Can Infer

- This Kindle is in the i.MX EPDC family and likely implements some variant of the Freescale/NXP
  framebuffer update API.
- Upstream kernels for this family support marker-based completion using:
  - `MXCFB_SEND_UPDATE`
  - `MXCFB_WAIT_FOR_UPDATE_COMPLETE`
- On this Kindle, the public control ioctls work, but the update submission ABI appears to differ
  from the layouts we tested.
- Because of that mismatch, we cannot currently use markers to ask "is the display ready now?"

## Known Risky Probe

The probe that appears to have caused the device restart was a timing experiment that submitted
sysfs refreshes over SSH.

Commands attempted:

```sh
ssh kindle 'echo 0 > /sys/devices/platform/imx_epdc_fb/mxc_epdc_update'
ssh kindle 'echo 0 > /sys/devices/platform/imx_epdc_fb/mxc_epdc_update; echo 0 > /sys/devices/platform/imx_epdc_fb/mxc_epdc_update'
```

There was also an earlier shell loop intended to repeat the same operation multiple times.

What happened:

- SSH stopped responding after the refresh submission test.
- The device became temporarily unreachable.
- The user observed that the Kindle restarted.

What we do not know yet:

- Whether the restart was caused by:
  - the specific `echo 0` sysfs refresh,
  - repeated refresh submission while the framework was already busy,
  - interaction with the framebuffer state left by the prior ioctl probes,
  - or a watchdog / UI service side effect.

Practical rule going forward:

- Do not use blind sysfs refresh timing loops as a probe method.
- Do not repeat consecutive `mxc_epdc_update` writes just to measure latency.
- Treat direct writes to `mxc_epdc_update` as potentially destabilizing until proven otherwise.

## Known Safe-ish Read-Only Inspection

These steps were read-only and did not appear to destabilize the device:

- `uname -a`
- `ls` / `find` under `/sys/devices/platform/imx_epdc_fb`
- `cat` of EPDC sysfs files
- `cat /proc/modules`
- `cat /proc/kallsyms | grep -i epdc`

These are reasonable first steps for future inspection.

## Open Questions

- Can we make ioctl updates work by using the Kindle PW3 / Wario `mxcfb_update_data` layout with:
  - `hist_bw_waveform_mode`
  - `hist_gray_waveform_mode`
  - no `virt_addr`
  - no dither fields?
- Does `MXCFB_WAIT_FOR_UPDATE_COMPLETE` succeed once called with a real marker struct produced by a
  successful ioctl update?
- Does `MXCFB_WAIT_FOR_UPDATE_SUBMISSION` work on this exact device and give us a useful "driver has
  accepted the work" fence distinct from visible completion?
- Is the current app always using the sysfs fallback path on this device?
- Does `lipc-set-prop -i com.lab126.powerd powerButton 1` still suspend correctly in the exact
  stripped-down runtime where `Lab126UI` is stopped for the game?
- No. User already tested this in the game runtime; it was a no-op.
- What mechanism fires the idle auto-sleep in that same runtime?
  - timer inside `powerd`? very likely yes, given `t1_timeout = 600`
  - `preventScreenSaver` / `touchScreenSaverTimeout` policy?
  - some other daemon or blanket integration?
- What user-triggerable API corresponds to the same suspend path if `powerButton` does not?
  - another `powerd` property?
  - a blanket / screensaver trigger followed by suspend?
  - direct kernel suspend plus explicit screensaver handling?
- Why does the raw `max77696-onkey` button no longer produce suspend in the game runtime even though:
  - `powerd` still owns the idle timers
  - `powerd` is configured against the raw input device
  - idle suspend still works?

## Conservative Next Steps For Future Probing

These are the next steps to try later, but they should not be run blindly from this document.

1. Recover the exact update ioctl ABI from the Kindle kernel/module.
   - This is now much narrower:
     - first try the PW2/PW3/Voyage Kindle `mxcfb_update_data` layout from KOReader / FBInk
     - do not keep probing random struct variants

2. Verify the expected wait ioctl argument type.
   - For this device family:
     - `WAIT_FOR_UPDATE_COMPLETE` should use `struct mxcfb_update_marker_data`
     - `WAIT_FOR_UPDATE_SUBMISSION` should use `u32`

3. Confirm whether the app is actually falling back to sysfs on-device.
   - This should be done by logging app-side path selection, not by stress-submitting updates.

4. Look for passive status signals.
   - Read-only observation during a normal app-driven refresh is still safer than ad hoc refresh
     loops.
   - But the passive surfaces found so far look weak:
     - `mxc_epdc_debug`
     - `mxc_epdc_regs`
     - `/proc/eink/...`
   - None of these currently look like a real queue-depth API.

5. Only after the correct send/update ABI is known, test marker-based completion with a single
   app-like update.
   - One explicit ABI.
   - One explicit update case.
   - Start with no waits at all.
   - Start with a tiny aligned region, not a broad sweep.
   - Only add `WAIT_FOR_UPDATE_SUBMISSION` or `WAIT_FOR_UPDATE_COMPLETE` after the send path is
     known not to wedge the device.
   - No timing loops.
   - No repeated sysfs writes.

6. After display markers are working, do one deliberate suspend-path test.
   - `powerButton` has already been tested in the game runtime and is not enough.
   - Next tests should focus on reproducing the already-working idle suspend path instead.

7. Investigate the already-working idle suspend path.
   - Observe `com.lab126.powerd state` and related LIPC events across an idle timeout.
   - Treat this as a likely cleaner route than trying to reproduce old `Lab126UI` button behavior.
   - Also observe `com.lab126.blanket` during the same transition.

## Working Hypothesis

The most likely path to reliable "display ready / display busy" information is:

- make ioctl-based update submission work on this specific Kindle using the Kindle PW3/Wario ABI,
- obtain a real `update_marker`,
- optionally wait for submission when we only need queue acceptance,
- wait for completion when we need true panel-idle / draw-done semantics.

Until that path is confirmed, the device should be treated as having no reliable queryable busy
state from the current app implementation.

## External References

These references informed the current understanding:

- Freescale/NXP EPDC header with `MXCFB_SEND_UPDATE` and
  `MXCFB_WAIT_FOR_UPDATE_COMPLETE`:
  - [mxcfb.h](https://git1.toradex.com/cgit/linux-toradex.git/tree/include/linux/mxcfb.h?h=v4.4.91&id=0eb553bf96e2c990d3bfccaa07da0863624c89ab)
- Linux i.MX EPDC driver showing wait/update ioctl handling:
  - [mxc_epdc_fb.c](https://coral.googlesource.com/linux-imx/%2B/refs/tags/11-2/drivers/video/fbdev/mxc/mxc_epdc_fb.c?autodive=0%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F)
- JavaFX Monocle EPD notes discussing marker-based waits on this driver family:
  - [EPDFrameBuffer.java](https://jar-download.com/artifacts/org.openjfx/javafx-graphics/17.0.0.1/source-code/com/sun/glass/ui/monocle/EPDFrameBuffer.java)
- Kindle-specific framebuffer header used by KOReader / FBInk:
  - [mxcfb-kindle.h](https://github.com/koreader/koreader-base/blob/master/ffi-cdecl/include/mxcfb-kindle.h)
- KOReader Kindle framebuffer logic:
  - [framebuffer_mxcfb.lua](https://github.com/koreader/koreader-base/blob/master/ffi/framebuffer_mxcfb.lua)
- KOReader Kindle power-management logic:
  - [powerd.lua](https://github.com/koreader/koreader/blob/master/frontend/device/kindle/powerd.lua)
- KOReader Kindle device table:
  - [device.lua](https://github.com/koreader/koreader/blob/master/frontend/device/kindle/device.lua)
- FBInk Kindle refresh / wait wrappers:
  - [fbink.c](https://github.com/NiLuJe/FBInk/blob/master/fbink.c)
  - [fbink.h](https://github.com/NiLuJe/FBInk/blob/master/fbink.h)
- Kindle serial / platform identifiers:
  - [kindle_tool.h](https://github.com/NiLuJe/KindleTool/blob/master/KindleTool/kindle_tool.h)
