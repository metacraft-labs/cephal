# metacraft-labs/cephal

This repository is Metacraft Labs' fork of [zortax/cephal](https://github.com/zortax/cephal), the
layout engine of [zgui](https://github.com/metacraft-labs/zgui). It is developed together with
Metacraft's zgui fork for [IsoNim](https://github.com/metacraft-labs/isonim), which drives zgui
through [isonim-zgui](https://github.com/metacraft-labs/isonim-zgui).

## Branches

- `isonim` is the product branch, the GitHub default, and the only branch that carries Metacraft
  changes. zgui's `isonim` branch pins an exact revision of it.
- `main` mirrors upstream's `main`. Nothing else is committed to it.

## Tracking upstream: merges, not rebases

The adaptation branch merges upstream's `main` into `isonim` at regular intervals. It does not
rebase onto it. This is a declared exception to the fork-branch rule in the Metacraft branching
policy: the changes here are developed for IsoNim and are not prepared as upstream pull requests,
so a linear patch stack has no reader, while a rebase would rewrite revisions that consumers pin.

To take upstream's latest work:

```sh
git fetch upstream
git switch isonim
git merge upstream/main
```

Resolve conflicts in favour of upstream's design where the two overlap, run the tests, and push
`isonim`.

## Changes made here

Every change carries a test. No pull requests or issues are opened on the upstream repository from
this fork.
