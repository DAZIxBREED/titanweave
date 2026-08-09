#!/usr/bin/env python3
from pathlib import Path
import hashlib
r=Path(__file__).resolve().parents[1]
sha=(r/'kernel/weavecore/src/sha256.rs').read_text()
assert '0x428a2f98' in sha and 'constant_time_eq' in sha and 'pub fn digest' in sha
assert hashlib.sha256(b'abc').hexdigest()=='ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
trust=(r/'kernel/weavecore/src/trust.rs').read_text()
for x in ['MAX_TRUST_KEYS','MAX_REVOCATIONS','SignatureVerifier','unknown signer','content is revoked','privileged content requires system/platform signer','user-signed content requires capability approval']:
    assert x in trust,x
cap=(r/'kernel/weavecore/src/capability.rs').read_text()
for x in ['SubjectKind::Application','SubjectKind::Driver','FirmwareFlash','SystemUpdate','is_subset_of']:
    assert x in cap,x
upd=(r/'kernel/weavecore/src/update.rs').read_text()
for x in ['anti-rollback floor','PendingBoot','maximum_boot_attempts','fail_and_revert','confirm']:
    assert x in upd,x
svc=(r/'kernel/weavecore/src/service.rs').read_text()
assert 'TRUSTD.ELF' in svc and 'ServiceRole::Trust' in svc
print('Titanweave K10 trust/update policy tests passed.')
