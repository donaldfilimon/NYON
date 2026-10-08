# Living V3 initialization: repair eligibility clarification

Status: **Proposed**. This dated clarification does not approve initialization or complete V3 mechanics, amend the eight frozen source documents, or qualify independent vectors.

The independently reviewed initialization design is `docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md`, SHA-256 `d305e685b58f37a3fadfb31325de48513a9e4b0fcdcb8a7faabe833e67c9b147`. This erratum clarifies its §4.3 sentence beginning “A first blocked repair” and its §13 Hull/Fleet coverage row. It preserves the reviewed source bytes.

A damaged initial asset has repair deadline 10 even if its location, ownership or combat state makes it ineligible. When an ordinary repair phase finds the asset ineligible, it emits no repair event, spends no Energy and preserves the existing deadline. Only an eligible, damaged, due attempt computes the next deadline from the previous deadline plus 10. An eligible attempt denied by physical Energy or hold availability still advances that deadline and spends no Energy. This follows `2026-10-01-living-v3-complete-schema.md` lines 2610–2622; it introduces no new eligibility guard.

Independent corpus coverage must include a damaged asset ineligible at tick 10, which later becomes eligible while its unchanged deadline is already due. The control must pin the preserved deadline, the first actual attempt, its funding outcome and the next deadline. An ordinary phase makes at most its specified attempt; it must not loop to catch up missed periods. Derive those expectations from the frozen producer contract and explicit fixture before testing a Rust executor.

Current evidence: preparatory design review identified the ambiguity. Partial: the added independent transition control has not been produced or qualified. Written-spec approval and complete-rules approval remain pending.
