# Dedicated-server verification

The native mod and optimized dedicated runtime were built against Horizon
`d3112570d0fbb6fc7715f53b414f4ee2f8d1ed59` with Rust 1.99.1 on Linux x86-64. The host was an
AMD Ryzen 5 5600X with 6 cores, 12 logical CPUs and 31 GiB of RAM. Server and wire clients ran on
the same workstation over loopback. The runtime used prepared Minecraft Java 26.3 data. The server and each
run's homes and world folder were disposable.

## Workload and measurements

Use the [README](README.md#verify) to build the export and run the tools. The full comparison rotates the
order of modes 0, 1, 2 and 3 over three rounds. Mode 0 loads the same native mod with no terrain views enabled.
Every trial starts from a copy of the same saved world, with seed 87387, view distance 6, simulation distance 3,
four clients, no other anticheat, no mob spawning, no random block ticks, and a 256-byte compression threshold.

Each phase lasts 12 seconds.

- **Join.** All four clients join at the same spawn.
- **Cold.** Clients travel at 18 blocks per second in four directions into new terrain from the same coordinates.
- **Warm.** Each client retraces its route, receiving previously seen columns.
- **Mining.** Four clients mine parallel rows in one shared region. The fixture spaces cuts far enough apart that
  one break's extra radius does not pre-reveal the next measured ore.

Chunk delivery counts packets actually received during the phase, including refreshes. The supplementary cold
test counts distinct and previously unseen columns per player as well. The benchmark counts complete compressed
packet frames, including length headers, and excludes unrelated packets. CPU is server process CPU time sampled
from Linux `/proc`. Peak RSS is the maximum sampled during the phase. Per-trial p95 and p99 are medians of the
server's `tick query` snapshots, with each snapshot covering the latest 100 ticks. They are rolling-window
estimates and can include ticks from the preceding phase. The report retains each snapshot and every phase's
delivered-chunk total.

The mining script matches the accepted block state change, the acknowledged action sequence and the actual
ore state on the wire. When the accepted change and the reveal are in one section update, they were queued
together. Where they occupy separate frames, the script records client receipt latency.

## Initial two-bot milestone

Nine-second windows, one disabled and one mode 1 trial on the same saved snapshot. Cold exploration delivered
33.3 chunks/s disabled and 54.9 chunks/s protected. Median sampled cold p95 rose from 2.45 to 3.45 ms.
These first counts include repeated columns and establish no acceptance threshold on their own.

## Interleaved comparison

Each cell is the median across three trials, with its minimum and maximum in brackets. Throughput is
delivered packet rate. CPU is seconds across all server threads in the 12-second phase.

| Mode | Phase | p95 ms | p99 ms | Chunks/s | Bytes/chunk | CPU seconds | Peak MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | join | 3.70 [3.30–8.70] | 11.10 [10.75–27.45] | 71.0 [71.0–71.0] | 4,439 [4,439–4,439] | 5.27 [4.96–5.39] | 527 [520–529] |
| 0 | cold | 6.60 [3.80–10.50] | 13.00 [10.15–14.70] | 67.5 [67.5–67.5] | 5,279 [5,274–5,280] | 14.39 [14.07–15.32] | 633 [629–633] |
| 0 | warm | 6.10 [5.25–10.10] | 8.40 [8.35–14.35] | 65.0 [65.0–65.0] | 4,476 [4,475–4,476] | 1.09 [1.08–1.29] | 638 [634–639] |
| 0 | mining | 1.70 [1.60–2.05] | 3.55 [2.15–4.60] | 16.2 [16.2–16.2] | 4,044 [4,044–4,044] | 0.47 [0.46–0.53] | 590 [587–599] |
| 1 | join | 3.45 [3.25–5.70] | 14.95 [11.25–17.40] | 71.0 [71.0–71.7] | 4,241 [4,241–4,242] | 5.46 [4.99–5.65] | 518 [518–523] |
| 1 | cold | 5.55 [4.60–8.80] | 11.15 [9.85–18.80] | 105.5 [104.8–108.2] | 4,949 [4,871–5,102] | 14.94 [14.82–15.66] | 627 [625–629] |
| 1 | warm | 7.95 [6.05–8.40] | 10.00 [8.45–12.30] | 65.0 [65.0–65.0] | 4,266 [4,266–4,266] | 1.18 [1.16–1.24] | 634 [632–637] |
| 1 | mining | 1.50 [1.45–1.65] | 2.20 [2.15–3.15] | 16.2 [16.2–16.2] | 3,870 [3,870–3,870] | 0.48 [0.48–0.48] | 581 [580–586] |
| 2 | join | 3.15 [2.75–3.25] | 11.65 [11.20–11.65] | 71.0 [71.0–71.0] | 12,142 [12,142–12,150] | 5.55 [5.32–5.55] | 534 [534–537] |
| 2 | cold | 6.75 [4.90–7.05] | 12.45 [12.40–20.55] | 104.8 [103.8–107.7] | 12,948 [12,884–13,162] | 16.50 [16.32–16.73] | 659 [656–661] |
| 2 | warm | 7.50 [7.15–7.95] | 9.80 [9.55–12.40] | 65.0 [65.0–65.0] | 12,243 [12,238–12,249] | 1.39 [1.39–1.41] | 667 [664–669] |
| 2 | mining | 1.55 [1.45–1.60] | 2.60 [2.05–3.85] | 16.2 [16.2–17.5] | 11,534 [11,530–11,546] | 0.50 [0.49–0.51] | 621 [621–628] |
| 3 | join | 3.15 [2.85–3.40] | 11.20 [10.90–11.90] | 71.0 [71.0–71.0] | 4,431 [4,422–4,432] | 5.29 [5.17–5.30] | 530 [529–533] |
| 3 | cold | 5.50 [4.90–5.85] | 9.45 [8.90–10.25] | 105.8 [104.5–107.8] | 5,060 [4,914–5,060] | 15.93 [15.87–16.36] | 648 [643–652] |
| 3 | warm | 7.10 [5.75–7.25] | 9.95 [7.85–10.85] | 65.0 [65.0–65.0] | 4,487 [4,478–4,489] | 1.34 [1.32–1.36] | 655 [652–660] |
| 3 | mining | 1.75 [1.45–1.80] | 2.35 [2.20–2.90] | 16.2 [16.2–16.2] | 3,848 [3,822–3,855] | 0.51 [0.50–0.52] | 610 [610–611] |

Mode 1's median p95 stayed within the 5% target during join, cold exploration and mining. Warm p95 rose
from 6.10 to 7.95 ms, about 30%, and missed the target. Cold delivery volume increased, which includes
the refresh packets the engine sends when adjacent terrain becomes resident. The distinct-column check
below isolates first-time terrain delivery.

The scripts accepted 120 breaks in each trial. In every protected mode all 360 ordinary mining reveals
arrived in the same section update as the accepted air change, meeting the next-tick queue target.
Request-to-receipt time includes client, input scheduling and server time. The maximum was 90.8 ms for
mode 1, 90.6 ms for mode 2 and 90.4 ms for mode 3.

For the matched traffic run on 49 identical saved columns, mode 2 sent 610,819 framed bytes and mode 3 sent
226,816. Mode 3 used **62.9% fewer bytes** in that sample. In the interleaved cold run its median compressed
bytes per delivered chunk fell from 12,948 to 5,060.

## First-time exploration delivery

Two additional disabled and mode 1 trials used the same saved snapshot, with order reversed in the second pair.
Every trial delivered 810 first-time columns in 12 seconds, or **67.5 per second**. Mode 1 met the 10% first-time
throughput target. It sent 442–445 additional refreshes per trial, compared with zero in the disabled trials.
Total framed chunk traffic rose from a median 4.28 MB to 6.18 MB, **44.3% more** for the same first-time delivery.
Per-chunk averages alone hide the refresh cost.

The second warm tick-query sample is wholly inside the warm phase. Taking only that sample, the median p95
was 2.60 ms disabled and 2.30 ms in mode 1. The phase-wide estimate in the table includes the transition out
of cold exploration and exceeded the target. The initial mode 0 trials used the same export with protection disabled.

## Disabled overhead against the plain engine

Two more interleaved pairs used a snapshot prepared by the same optimized engine without the mod loaded.
The plain and disabled exports each delivered 71.0 chunks/s at join and 67.5 first-time chunks/s in cold
exploration in every trial. Cold CPU medians were 14.46 s plain and 14.71 s disabled, a 1.7% increase.
Cold p95 ranged from 4.80 to 5.90 ms plain and 4.35 to 8.30 ms disabled. The ranges overlap, and one disabled
trial exceeded the plain range. The sample does not establish tick-time neutrality. Cold peak RSS medians
were 608 MiB plain and 634 MiB disabled.

## Packet and gameplay results

The unmodified Minecraft Java 26.3 client joined the protected dedicated export and rendered its live terrain
in a fresh Xvfb session. The remaining checks used stock protocol clients to observe individual packets.

- Two protected users saw deepslate at the lit ore and east-facing chest below Y=0 and stone at the ore above Y=0.
  Their initial columns omitted the concealed chest record. The operator saw the authoritative states and the
  chest record. Removing and restoring operator status refreshed that client's projection both ways.
- Glass, a single slab and water left nearby ore visible. Lava left it visible with lava occlusion off and concealed
  it with lava occlusion on.
- The out-of-range dig was acknowledged without making the ore visible. A valid break at the chunk border
  exposed the real ore and chest to both protected clients. Resealing and fill edits reapplied the projection.
  The saved Anvil chunk kept the authoritative lit ore, the east-facing chest and three diamonds in its inventory.
  Destroying the chest dropped the diamonds. A survival dig with a diamond pickaxe dropped redstone.
- The saved far-ring column (12, 0) delivered fake deepslate and no chest record to the protected client and the
  real chest and record to the operator. Both stayed at spawn. The pinned engine's decoded packet tests also
  passed for resident neighbor arrival invalidating a saved projection and for a wrapped cross-dimension view.
- Mode 2 varied within all 59 sampled layers. Mode 3 was uniform in each of the 59 layers. Both used the configured
  decoys, suppressed the replacement chest's metadata, respected the rounded Y=0 limit, and repeated the
  projection after unload, revisit and a Nether round trip. The Nether used netherrack for buried ancient debris.
  Selecting a chest as a mode 2 decoy named the dimension and setting at startup and refused login.

The sustained mining run reached the **1,024-entry** extra reveal cap after break 57. All 72 ordinary adjacent ore
reveals still reached both viewers, including after capacity. Before capacity, the still-covered chest's true state
was sent only to the miner. After capacity, another still-covered chest received no extra state. Breaking its
immediate seal then sent its true state to both viewers. With this engine revision, **none of the chest reveals
carried a separate chest update tag**. The engine agent's separate fix
`e553c2c81940457ffcdaf1727bc50b87006df0d7` passed state-before-tag assertions for ordinary and retained
radius reveals. The dedicated results and benchmarks above use the original requested pin.
