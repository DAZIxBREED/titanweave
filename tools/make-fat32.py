#!/usr/bin/env python3
"""Build Titanweave K10's deterministic FAT32 bootstrap volume.

This intentionally emits only 8.3 directory entries because the K6 reader is
small, bounded, and read-only. Long-file-name support belongs in the later full
filesystem service rather than the kernel bootstrap parser.
"""
from __future__ import annotations

import argparse
import math
import struct
from dataclasses import dataclass
from pathlib import Path

SECTOR = 512
TOTAL_SECTORS = 131072  # 64 MiB; enough clusters to be FAT32 by specification
RESERVED = 32
FAT_COUNT = 2
FAT_SECTORS = 1024
SECTORS_PER_CLUSTER = 1
FIRST_DATA = RESERVED + FAT_COUNT * FAT_SECTORS
EOC = 0x0FFFFFFF
MEDIA = 0xF8


@dataclass
class Node:
    short_name: bytes
    data: bytes
    attr: int = 0x20
    first_cluster: int = 0


def short(name: str) -> bytes:
    stem, dot, ext = name.upper().partition(".")
    if not stem or len(stem) > 8 or len(ext) > 3:
        raise ValueError(f"not an 8.3 name: {name}")
    return stem.encode("ascii").ljust(8, b" ") + ext.encode("ascii").ljust(3, b" ")


def dirent(name: bytes, attr: int, cluster: int, size: int) -> bytes:
    if len(name) != 11:
        raise ValueError("directory name must be exactly 11 bytes")
    entry = bytearray(32)
    entry[0:11] = name
    entry[11] = attr
    struct.pack_into("<H", entry, 20, (cluster >> 16) & 0xFFFF)
    struct.pack_into("<H", entry, 26, cluster & 0xFFFF)
    struct.pack_into("<I", entry, 28, size)
    return bytes(entry)


def dot_entry(parent: bool, cluster: int) -> bytes:
    name = (b".." if parent else b".").ljust(11, b" ")
    return dirent(name, 0x10, cluster, 0)


def cluster_offset(cluster: int) -> int:
    return (FIRST_DATA + (cluster - 2) * SECTORS_PER_CLUSTER) * SECTOR


def write_cluster(image: bytearray, cluster: int, data: bytes) -> None:
    capacity = SECTOR * SECTORS_PER_CLUSTER
    if len(data) > capacity:
        raise ValueError("single cluster payload overflow")
    offset = cluster_offset(cluster)
    image[offset : offset + len(data)] = data


def allocate_file(fat: list[int], next_cluster: int, data: bytes) -> tuple[int, int]:
    clusters = max(1, math.ceil(len(data) / (SECTOR * SECTORS_PER_CLUSTER)))
    first = next_cluster
    for index in range(clusters):
        cluster = first + index
        fat[cluster] = EOC if index + 1 == clusters else cluster + 1
    return first, first + clusters


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--userspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    files = [
        Node(short("INIT.ELF"), (args.userspace / "INIT.ELF").read_bytes()),
        Node(short("LOGD.ELF"), (args.userspace / "LOGD.ELF").read_bytes()),
        Node(short("CONSOL.ELF"), (args.userspace / "CONSOL.ELF").read_bytes()),
        Node(short("DISPLAYD.ELF"), (args.userspace / "DISPLAYD.ELF").read_bytes()),
        Node(short("SHELL.ELF"), (args.userspace / "SHELL.ELF").read_bytes()),
        Node(short("ARCHIVE.ELF"), (args.userspace / "ARCHIVE.ELF").read_bytes()),
        Node(short("TRUSTD.ELF"), (args.userspace / "TRUSTD.ELF").read_bytes()),
        Node(short("DRIVERD.ELF"), (args.userspace / "DRIVERD.ELF").read_bytes()),
        Node(
            short("SERVICES.CFG"),
            b"init=C:\\SYSTEM\\SERVICES\\INIT.ELF\r\n"
            b"logging=C:\\SYSTEM\\SERVICES\\LOGD.ELF\r\n"
            b"console=C:\\SYSTEM\\SERVICES\\CONSOL.ELF\r\n"
            b"display=C:\\SYSTEM\\SERVICES\\DISPLAYD.ELF\r\n"
            b"shell=C:\\SYSTEM\\SERVICES\\SHELL.ELF\r\n"
            b"archive=C:\\SYSTEM\\SERVICES\\ARCHIVE.ELF\r\n"
            b"trust=C:\\SYSTEM\\SERVICES\\TRUSTD.ELF\r\n"
            b"drivers=C:\\SYSTEM\\SERVICES\\DRIVERD.ELF\r\n",
        ),
    ]
    boot_cmd = Node(
        short("BOOT.CMD"),
        b"services\r\ndir C:\\SYSTEM\\SERVICES\r\nps\r\nuptime\r\n",
    )

    image = bytearray(TOTAL_SECTORS * SECTOR)
    fat_entries = FAT_SECTORS * SECTOR // 4
    fat = [0] * fat_entries
    fat[0] = 0x0FFFFF00 | MEDIA
    fat[1] = EOC

    root_cluster = 2
    system_cluster = 3
    services_cluster = 4
    fat[root_cluster] = EOC
    fat[system_cluster] = EOC
    fat[services_cluster] = EOC

    next_cluster = 5
    for node in files + [boot_cmd]:
        node.first_cluster, next_cluster = allocate_file(fat, next_cluster, node.data)

    # FAT32 BPB / extended boot record.
    boot = bytearray(SECTOR)
    boot[0:3] = b"\xEB\x58\x90"
    boot[3:11] = b"TITANWVE"
    struct.pack_into("<H", boot, 11, SECTOR)
    boot[13] = SECTORS_PER_CLUSTER
    struct.pack_into("<H", boot, 14, RESERVED)
    boot[16] = FAT_COUNT
    struct.pack_into("<H", boot, 17, 0)
    struct.pack_into("<H", boot, 19, 0)
    boot[21] = MEDIA
    struct.pack_into("<H", boot, 22, 0)
    struct.pack_into("<H", boot, 24, 63)
    struct.pack_into("<H", boot, 26, 255)
    struct.pack_into("<I", boot, 28, 0)
    struct.pack_into("<I", boot, 32, TOTAL_SECTORS)
    struct.pack_into("<I", boot, 36, FAT_SECTORS)
    struct.pack_into("<H", boot, 40, 0)
    struct.pack_into("<H", boot, 42, 0)
    struct.pack_into("<I", boot, 44, root_cluster)
    struct.pack_into("<H", boot, 48, 1)
    struct.pack_into("<H", boot, 50, 6)
    boot[64] = 0x80
    boot[66] = 0x29
    struct.pack_into("<I", boot, 67, 0x544B3601)
    boot[71:82] = b"TITANWEAVE "
    boot[82:90] = b"FAT32   "
    boot[510:512] = b"\x55\xAA"
    image[0:SECTOR] = boot
    image[6 * SECTOR : 7 * SECTOR] = boot

    fsinfo = bytearray(SECTOR)
    struct.pack_into("<I", fsinfo, 0, 0x41615252)
    struct.pack_into("<I", fsinfo, 484, 0x61417272)
    struct.pack_into("<I", fsinfo, 488, fat_entries - next_cluster)
    struct.pack_into("<I", fsinfo, 492, next_cluster)
    struct.pack_into("<I", fsinfo, 508, 0xAA550000)
    image[SECTOR : 2 * SECTOR] = fsinfo

    fat_bytes = bytearray(FAT_SECTORS * SECTOR)
    for index, value in enumerate(fat):
        struct.pack_into("<I", fat_bytes, index * 4, value)
    for fat_index in range(FAT_COUNT):
        start = (RESERVED + fat_index * FAT_SECTORS) * SECTOR
        image[start : start + len(fat_bytes)] = fat_bytes

    root_entries = [
        dirent(short("SYSTEM"), 0x10, system_cluster, 0),
    ]
    system_entries = [
        dot_entry(False, system_cluster),
        dot_entry(True, root_cluster),
        dirent(short("SERVICES"), 0x10, services_cluster, 0),
        dirent(boot_cmd.short_name, boot_cmd.attr, boot_cmd.first_cluster, len(boot_cmd.data)),
    ]
    service_entries = [
        dot_entry(False, services_cluster),
        dot_entry(True, system_cluster),
        *[
            dirent(node.short_name, node.attr, node.first_cluster, len(node.data))
            for node in files
        ],
    ]

    write_cluster(image, root_cluster, b"".join(root_entries) + b"\0" * 32)
    write_cluster(image, system_cluster, b"".join(system_entries) + b"\0" * 32)
    write_cluster(image, services_cluster, b"".join(service_entries) + b"\0" * 32)

    for node in files + [boot_cmd]:
        remaining = node.data
        cluster = node.first_cluster
        while True:
            chunk = remaining[:SECTOR]
            write_cluster(image, cluster, chunk)
            remaining = remaining[SECTOR:]
            if not remaining:
                break
            cluster = fat[cluster]

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(image)
    print(f"wrote {args.output} ({len(image)} bytes, next free cluster {next_cluster})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
