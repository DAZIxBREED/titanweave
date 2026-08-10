#!/usr/bin/env python3
"""Apply the K15.8 graph-engine integration to a qualified K15.7 tree.

This patcher intentionally performs exact-anchor replacements. It refuses to
modify a tree that does not match the expected frozen K15.7 baseline, avoiding
whole-file rewrites of large kernel/userspace sources.
"""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]


def patch(path: str, old: str, new: str) -> None:
    target = ROOT / path
    text = target.read_text()
    if new in text:
        print(f"already integrated: {path}")
        return
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"K15.8 integration anchor mismatch in {path}: expected 1, found {count}")
    target.write_text(text.replace(old, new, 1))
    print(f"patched: {path}")


# Register the graph module without disturbing the huge K14/K15 kernel entry.
patch(
    "kernel/weavecore/src/main.rs",
    "mod forgeaudio_transport;\nmod gpu_topology;",
    "mod forgeaudio_transport;\nmod forgeaudio_graph_engine;\nmod gpu_topology;",
)

# K15.8 deliberately reuses the already-privileged ForgeAudioD qualification
# call. K15.7 must pass first; only then is the bounded graph executed. Any
# graph failure returns ERROR_NOT_READY, causing ForgeAudioD to fail closed.
patch(
    "kernel/weavecore/src/syscalls.rs",
    '''                    serial::println(format_args!(
                        "[K15LR] ForgeAudio lock-free transport ready: version=1 block_bytes={} ring_slots={} command_depth={} SPSC=true atomics=true allocation_free=true server_persistent=true",
                        AUDIO_TRANSPORT_BLOCK_BYTES, titanweave_forgeaudio_abi::AUDIO_TRANSPORT_RING_SLOTS,
                        titanweave_forgeaudio_abi::AUDIO_TRANSPORT_COMMAND_DEPTH
                    ));
                    1
''',
    '''                    serial::println(format_args!(
                        "[K15LR] ForgeAudio lock-free transport ready: version=1 block_bytes={} ring_slots={} command_depth={} SPSC=true atomics=true allocation_free=true server_persistent=true",
                        AUDIO_TRANSPORT_BLOCK_BYTES, titanweave_forgeaudio_abi::AUDIO_TRANSPORT_RING_SLOTS,
                        titanweave_forgeaudio_abi::AUDIO_TRANSPORT_COMMAND_DEPTH
                    ));
                    let graph = match crate::forgeaudio_graph_engine::run_qualification() {
                        Ok(report) => report,
                        Err(error) => {
                            serial::println(format_args!("[FAIL] K15.8 ForgeAudio Graph Engine qualification failed: {error}"));
                            return encode_error(ERROR_NOT_READY);
                        }
                    };
                    serial::println(format_args!(
                        "[K15OK] K15.8 ForgeAudio Graph Engine qualified: version={} generation={} nodes={} edges={} blocks={} frames={} Input=true Output=true Gain=true Mixer=true Splitter=true ChannelMapper=true FormatConverter=true Meter=true allocation_free=true bounded=true deterministic_order=true",
                        graph.version, graph.generation, graph.nodes, graph.edges, graph.blocks, graph.frames
                    ));
                    serial::println(format_args!(
                        "[K15GR] ForgeAudio Graph Engine ready: max_nodes={} max_inputs={} runtime_locks=0 topology_compile_rt=false sample_accurate_switching=false resampling=false",
                        crate::forgeaudio_graph_engine::MAX_GRAPH_NODES,
                        crate::forgeaudio_graph_engine::MAX_GRAPH_INPUTS
                    ));
                    1
''',
)

# A successful ServerQualify now proves both inherited K15.7 and K15.8. Keep
# the K15.7 banner, then emit a distinct K15.8 userspace milestone.
patch(
    "userspace/forgeaudiod/forgeaudiod.S",
    '''    TW_WRITE TW_CONSOLE_HANDLE, transport_ready_msg, TRANSPORT_READY_MSG_LEN
    ret
''',
    '''    TW_WRITE TW_CONSOLE_HANDLE, transport_ready_msg, TRANSPORT_READY_MSG_LEN
    TW_WRITE TW_CONSOLE_HANDLE, graph_ready_msg, GRAPH_READY_MSG_LEN
    ret
''',
)
patch(
    "userspace/forgeaudiod/forgeaudiod.S",
    '''transport_heartbeat_msg:
    .ascii "[USER] [forgeaudiod] K15.7 post-isolation heartbeat: sequence=2 server_alive=true"
.set TRANSPORT_HEARTBEAT_MSG_LEN, . - transport_heartbeat_msg
''',
    '''transport_heartbeat_msg:
    .ascii "[USER] [forgeaudiod] K15.7 post-isolation heartbeat: sequence=2 server_alive=true"
.set TRANSPORT_HEARTBEAT_MSG_LEN, . - transport_heartbeat_msg
graph_ready_msg:
    .ascii "[USER] [forgeaudiod] K15.8 graph engine ready: nodes=8 Input=true Output=true Gain=true Mixer=true Splitter=true ChannelMapper=true FormatConverter=true Meter=true"
.set GRAPH_READY_MSG_LEN, . - graph_ready_msg
''',
)

# Add the K15.8 structural test to the integrated source validator while
# retaining every earlier gate's tests.
patch(
    "tools/validate-source.sh",
    '''python3 "$ROOT/tools/test-k15-7-forgeaudio-lockfree.py"
echo "Titanweave K1-K15.7 integrated source validation passed; K15.7 runtime qualification pending."
''',
    '''python3 "$ROOT/tools/test-k15-7-forgeaudio-lockfree.py"
python3 "$ROOT/tools/test-k15-8-forgeaudio-graph.py"
echo "Titanweave K1-K15.8 integrated source validation passed; K15.8 runtime qualification pending."
''',
)

print("Titanweave K15.8 ForgeAudio Graph Engine integration applied.")
