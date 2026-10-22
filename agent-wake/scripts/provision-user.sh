#!/usr/bin/env bash
set -euo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
	exec sudo -- "$0" "$@"
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
env_file="${AGENT_WAKE_ENV_FILE:-/etc/agent-wake.env}"
service_name="${AGENT_WAKE_SERVICE_NAME:-agent-wake.service}"
bun_bin="${BUN_BIN:-$(command -v bun 2>/dev/null || true)}"

if [[ -z "$bun_bin" ]]; then
	for candidate in /home/admin/.bun/bin/bun /usr/local/bin/bun /usr/bin/bun; do
		if [[ -x "$candidate" ]]; then
			bun_bin="$candidate"
			break
		fi
	done
fi
if [[ -z "$bun_bin" || ! -x "$bun_bin" ]]; then
	printf 'Bun is required to hash the password.\n' >&2
	exit 1
fi

usage() {
	cat <<'EOF'
Usage:
  sudo ./scripts/provision-user.sh [USERNAME]
  printf '%s' "$PASSWORD" | sudo ./scripts/provision-user.sh --stdin [USERNAME]

Creates or replaces the single test-host account. The password is never accepted
as a command-line argument and is not printed. Existing non-auth environment
settings in the env file are preserved.
EOF
}

username="${AGENT_WAKE_USERNAME:-}"
password_stdin=false
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
	usage
	exit 0
fi
if [[ "${1:-}" == "--stdin" ]]; then
	password_stdin=true
	shift
fi
if [[ $# -gt 0 ]]; then
	username="$1"
	shift
fi
if [[ $# -gt 0 ]]; then
	usage >&2
	exit 2
fi
if [[ -z "$username" ]]; then
	read -r -p 'Username: ' username
fi
if [[ ! "$username" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{2,63}$ ]]; then
	printf 'Username must be 3–64 characters using letters, numbers, _, ., or -.\n' >&2
	exit 2
fi

if [[ "$password_stdin" == true ]]; then
	IFS= read -r password || true
else
	read -r -s -p 'Password (12+ characters): ' password
	printf '\n'
	read -r -s -p 'Confirm password: ' confirmation
	printf '\n'
	if [[ "$password" != "$confirmation" ]]; then
		printf 'Passwords do not match.\n' >&2
		exit 2
	fi
fi
if [[ "${#password}" -lt 12 ]]; then
	printf 'Password must be at least 12 characters.\n' >&2
	exit 2
fi

password_hash="$(printf '%s' "$password" | "$bun_bin" -e 'const password = await Bun.stdin.text(); if (password.length < 12) process.exit(2); process.stdout.write(await Bun.password.hash(password));')"
unset password confirmation

install -d -m 0750 "$(dirname -- "$env_file")"
temp_file="$(mktemp "${env_file}.tmp.XXXXXX")"
cleanup() { rm -f -- "$temp_file"; }
trap cleanup EXIT

if [[ -f "$env_file" ]]; then
	awk '!/^AGENT_WAKE_USERNAME=/ && !/^AGENT_WAKE_PASSWORD=/ && !/^AGENT_WAKE_PASSWORD_HASH=/' "$env_file" > "$temp_file"
else
	: > "$temp_file"
fi
printf 'AGENT_WAKE_USERNAME=%s\n' "$username" >> "$temp_file"
printf 'AGENT_WAKE_PASSWORD_HASH=%s\n' "$password_hash" >> "$temp_file"
install -o root -g root -m 0600 "$temp_file" "$env_file"
unset password_hash

systemctl restart "$service_name"
if ! systemctl is-active --quiet "$service_name"; then
	printf 'Account was written, but %s did not become active.\n' "$service_name" >&2
	systemctl --no-pager --full status "$service_name" >&2 || true
	exit 1
fi
printf 'Provisioned account %q on %s.\n' "$username" "$service_name"
printf 'The password was hashed at rest; it was not printed.\n'
