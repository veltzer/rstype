# Future Mode Ideas

This chapter collects ideas for additional typing modes beyond the five currently
implemented (Forward, Stop, Correct, Sudden Death, Blind). These are candidates
for future implementation — not promises. They are grouped by what kind of skill
or experience they target.

## Current Modes (for reference)

| Mode | Concept |
|------|---------|
| **Forward** | Errors are shown but don't block progress |
| **Stop** | Cursor blocks until you hit the correct key |
| **Correct** | Errors advance but you must backspace and fix them before finishing |
| **Sudden Death** | One mistake resets you to the start |
| **Blind** | No visual feedback — all typed chars shown as dots |

## Accuracy / Penalty Modes

### Rewind

A wrong key sends you back N characters (e.g. 3–5) rather than all the way to the
start like Sudden Death. A middle ground between Correct and Sudden Death — the
penalty hurts but isn't catastrophic.

### Three Strikes

You get N lives. Each mistake costs one. Lose them all and the session restarts.
Adds tension without the brutality of Sudden Death, and gives the user a visible
"health" indicator in the status bar.

### Decay

Each mistake adds a time penalty (e.g. +2 seconds) to your final score. You can
keep going, but errors are costly. Encourages a risk/reward calculation: is it
faster to backspace-and-fix or to eat the penalty?

## Speed / Pressure Modes

### Countdown

You have a fixed time limit (e.g. 30s, 60s, 120s). Type as much as you can before
time runs out. Measures raw throughput under pressure. This is the standard mode
in most web-based typing trainers (monkeytype, 10fastfingers).

### Accelerating

You must maintain a minimum WPM that gradually increases over the session. Fall
below the threshold and you fail. Tests how fast you can sustain accuracy — a
moving target rather than a fixed one.

### Sprint

Short bursts (single words or short phrases) back-to-back, with per-word timing
rather than whole-text timing. Focuses on burst speed and reaction time rather
than sustained typing.

## Learning / Training Modes

### Mirror

The text is displayed reversed (or the keyboard mapping is flipped). A
neuroplasticity challenge — forces conscious processing rather than muscle
memory. Probably a novelty, but interesting.

### Weighted

After a round, the app identifies your weakest keys (highest error rate or
slowest reaction time) and generates the next round's text emphasizing those
characters. Targeted weakness training — makes the tool genuinely adaptive
rather than just presenting fixed content.

### Rhythm

A metronome or visual pulse sets the pace. You must type one character per beat.
Trains consistent cadence rather than raw speed. Useful because inconsistent
rhythm (fast bursts followed by pauses) is a common typing flaw even among
fast typists.

### No Backspace

Like Forward, but Backspace is explicitly disabled and errors are permanent.
Forces you to commit and move forward — trains confidence and discourages the
habit of second-guessing every keystroke.

## Endurance / Challenge Modes

### Marathon

An endless stream of text. No defined end — just tracks how long you can sustain
a target WPM/accuracy before fatigue sets in. Good for stamina training and for
observing how performance degrades over time.

### Survival

Combines Countdown + Sudden Death: you have a timer, and each correct word adds
time to the clock. One mistake ends the run. How long can you survive? Gamifies
the session in a way that rewards both speed and accuracy simultaneously.

### Zen

No timer, no error tracking, no score. Just type. For warming up or practicing
without pressure. The anti-mode — a deliberate counterweight to Sudden Death.

## Priority

If only a few of these are implemented, the highest-impact additions would be:

1. **Countdown** — it's the expected default in most typing trainers and would
   make rstype feel familiar to users coming from the web.
2. **Weighted** — turns rstype into an adaptive trainer rather than a fixed
   content player. Biggest step up in actual pedagogical value.
3. **Zen** — a cheap-to-implement complement to the high-pressure modes, and
   useful as a warm-up mode before a "real" session.
