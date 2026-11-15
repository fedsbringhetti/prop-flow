#!/usr/bin/env bash
# Svuota la target directory di Cargo, dopo averne mostrato la dimensione e averne
# chiesto conferma.
#
# **Perché serve.** Cargo non cancella mai una build superata. L'hash nei nomi
# degli artefatti (`libfoo-53d0b5347d5c3856.rlib`) dipende da compilatore,
# `Cargo.lock`, feature e profilo, **non dai sorgenti**: ogni cambio di toolchain
# (`rust-toolchain.toml`), ogni aggiornamento del lock, ogni combinazione nuova di
# feature lascia dietro una serie completa di librerie e di binari di test, che
# nessuna build successiva riusa e nessuna rimuove. In un altro progetto la
# cartella era arrivata a 132 GB, con 46 copie della stessa libreria, e il disco a
# 1 GB libero.
#
# Scelte che sembrano semplificabili e non lo sono:
#
# - **La cartella si chiede a Cargo, non si indovina.** `cargo metadata` dà la
#   target directory effettiva, che tiene conto sia di `CARGO_TARGET_DIR` sia di
#   `build.target-dir` in `.cargo/config.toml`: è quella che `cargo clean` svuota
#   davvero. Leggere solo `CARGO_TARGET_DIR`, o dare per scontato `target/`,
#   significa misurare una cartella e cancellarne un'altra. Per la stessa ragione
#   `cargo metadata` e `cargo clean` girano dalla stessa cartella e nello stesso
#   processo, quindi con lo stesso ambiente — compreso ciò che `just` carica da
#   `.env`.
# - **Il campo si estrae con `sed`, non con `jq`**: il progetto non usa `jq` da
#   nessuna parte, e un comando di pulizia non deve chiedere un'installazione. Il
#   JSON di Cargo sta su una riga e non scappa i caratteri non ASCII; l'unico
#   percorso che `sed` non sa leggere è uno con `\` o `"`, che il JSON scappa, e lì
#   lo script si ferma invece di indovinare. Il percorso estratto serve solo a
#   misurare e a spiegare: che cosa si cancella lo decide `cargo clean` da sé.
# - **`cargo clean`, non `cargo sweep`**: non richiede di installare niente, e
#   svuotare la cartella è tutto ciò che questo comando promette. Una pulizia
#   selettiva, per età o per toolchain, sarebbe un altro strumento.
# - **La conferma è scritta a mano, non con `[confirm]` di `just`**: deve poter dire
#   quanto spazio sta per liberare e se la cartella è condivisa con altri progetti,
#   e `[confirm]` mostra solo una frase fissa. Il default è no, e senza un terminale
#   (`read` trova EOF, come in CI) vale no: un comando che cancella decine di GB non
#   deve partire perché nessuno ha risposto.
# - **Alla fine non si stampa un comando di ricostruzione**: nessuna ricetta
#   presuppone un binario già compilato — `api`, `worker` e `migrate` passano da
#   `cargo run`, la suite da `cargo test` — quindi la build successiva si rifà da
#   sé, solo partendo da fredda.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

metadata="$(cargo metadata --format-version 1 --no-deps)"

# Un campo stringa di primo livello del JSON di `cargo metadata`. Vuoto se manca
# o se il valore contiene caratteri scappati: vedi sopra.
json_field() {
  printf '%s\n' "$metadata" | sed -nE "s/.*\"$1\":\"([^\"\\\\]*)\".*/\1/p"
}

target_dir="$(json_field target_directory)"
workspace_root="$(json_field workspace_root)"
build_dir="$(json_field build_directory)"

if [[ -z "$target_dir" || -z "$workspace_root" ]]; then
  echo "error: could not read target_directory from 'cargo metadata'." >&2
  echo "       A path containing a backslash or a double quote is not supported." >&2
  exit 1
fi

if [[ ! -d "$target_dir" ]]; then
  echo "$target_dir does not exist: nothing to sweep"
  exit 0
fi

# I percorsi fisici, con i link simbolici risolti: un `target` che è un link a una
# cartella altrove è condiviso quanto un `CARGO_TARGET_DIR` esplicito.
target_phys="$(cd -P "$target_dir" && pwd -P)"
root_phys="$(cd -P "$workspace_root" && pwd -P)"

# Una target directory che contiene il progetto — `CARGO_TARGET_DIR=..`, o la
# radice stessa — farebbe cancellare a `cargo clean` i sorgenti. Improbabile, ma
# per un comando che cancella costa due righe.
if [[ "$root_phys" == "$target_phys" || "$root_phys" == "$target_phys"/* ]]; then
  echo "error: the target directory $target_phys contains the project itself." >&2
  echo "       Refusing to run cargo clean on it." >&2
  exit 1
fi

shared=0
if [[ "$target_phys" != "$root_phys"/* ]]; then
  shared=1
fi

# `awk` e non `cut -f1`: il `du` di macOS allinea la dimensione con spazi in testa.
size="$(du -sh "$target_dir" | awk '{print $1}')"

echo "Cargo target directory: $target_dir ($size)"
echo ""
echo "It holds everything Cargo has built for this workspace: debug and release"
echo "builds, test binaries, incremental caches, rust-analyzer's check builds — and"
echo "every superseded copy of them, which Cargo never removes. None of it is source"
echo "and all of it can be rebuilt, but the next build starts cold."
echo ""
echo "Contents:"
entries=()
shopt -s nullglob dotglob
for entry in "$target_dir"/*; do
  entries+=("${entry##*/}")
done
shopt -u nullglob dotglob
if ((${#entries[@]} > 0)); then
  (cd "$target_dir" && du -sh -- "${entries[@]}") | sed 's/^/  /'
fi
echo "Anything else stored in there (logs, scratch files) is deleted too."

if [[ -n "$build_dir" && "$build_dir" != "$target_dir" ]]; then
  echo ""
  echo "Note: build-dir is set to $build_dir, separate from the target directory;"
  echo "its size is not included above."
fi

if ((shared)); then
  echo ""
  echo "WARNING: this directory is outside the project ($root_phys),"
  echo "so it is probably shared with other projects: cargo clean will delete"
  echo "their builds too."
fi

echo ""
read -r -p "Delete it? [y/N] " answer || answer=""

case "$answer" in
  y | Y | yes | YES) ;;
  *)
    echo "nothing deleted"
    exit 0
    ;;
esac

cargo clean
echo "Swept $target_dir. Nothing to rebuild by hand: every recipe builds through cargo."
