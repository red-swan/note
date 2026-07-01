# note

Quick CLI notes synced across machines via S3.

## Install

```
cargo install --path .
```

This builds and copies the binary to `~/.cargo/bin/note`. Make sure `~/.cargo/bin` is on your `PATH`.

Re-run this command after any code change to update the installed binary — `cargo build` alone only builds into `./target`, it does not touch `PATH`.

## Setup (per machine)

Point the tool at a shared S3 object (same path on every machine you want synced):

```
note config s3://<bucket>/<prefix>/notes.temp
```

Requires the `aws` CLI installed and authenticated (e.g. via `aws configure` or SSO) with read/write access to that bucket/key.

## Usage

```
note <text>          # append a note (syncs with S3 first if remote changed)
note open            # sync, then open the notes file
note push            # sync + push local notes file as-is, no note added (e.g. after hand-editing)
note push --force    # force-push local notes file, skipping pull/conflict check
```

## Files

- `~/.config/note/config` — stores the configured `s3_path` and `last_synced` (S3's server-side `LastModified`, not local clock).
- `~/.config/note/notes.temp` — the actual notes file.
- `~/.config/note/notes.temp.remote` — written only on conflict (see below), holds the remote copy for comparison.

## Sync model

Every write does: check remote `LastModified` → pull if it changed since our last sync → append → re-check remote right before pushing → push.

If the remote changed again in that narrow window between pull and push (i.e. another machine wrote at nearly the same time), the push is aborted rather than silently overwritten. You'll see a diff against `notes.temp.remote` and a prompt to resolve manually, then run `note push --force`. This tool assumes single-user, not-quite-simultaneous use — true concurrent writes from two machines aren't merged automatically.
