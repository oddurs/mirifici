# Security policy

`mirifici` is a numerics library with no I/O, no allocation and no `unsafe` code beyond the
`bytemuck` trait impls. The most likely security-relevant defect is a panic or a wrong
result on adversarial input, for example from `FromStr` on untrusted strings.

Please report suspected vulnerabilities privately through GitHub's
[security advisory form](https://github.com/oddurs/mirifici/security/advisories/new)
rather than a public issue. Include the input that triggers the problem and the crate
version. You should get a reply within a week.

Supported versions: the latest published release.
