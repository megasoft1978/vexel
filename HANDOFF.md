# Handoff

WO-M4-01 revision 1 is accepted and fast-forward merged to `main` at `c7aa4c9fa2966b90deaa08e4cb848c78db29de1b`. It adds bounded basic shapes, affine transforms, nested groups, and repeatable corpus reporting with measured binary sizes. Seven fresh read-only reviews ended in a pass after six substantive findings were fixed.

Verification: CTest 11/11, Python 192/192, Debug/Release, ASan/UBSan, native smoke and 20-frame replay pass. Seeded shape fuzz passed 1,000,000 cases. Corpus: 58 files/116 trials, 2 previewable, 56 unsupported, no limits/crashes. The 100 MiB preview gate fails; maximum main-thread step and RSS gates pass. The data measures open/index, not full-render parity.

WO-M4-02 revision 1 was superseded before execution for a fuzz artifact path correction. Revision 2 is frozen and active at `b6c496f3ca9e1d22946015ad74757d2fefea0097`; spec hash `2fd9975b8de99eb7e2149011839ccdc163f76341b9bb9b820435188ea462580d`.
