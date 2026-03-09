#!/usr/bin/env bash
# install.sh: Installer for Legion (https://github.com/ssilvius/legion)
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ssilvius/legion/main/install.sh | bash
#   curl -fsSL ... | bash -s v0.2.0
#   LEGION_VERSION=v0.2.0 bash install.sh

set -euo pipefail

REPO="ssilvius/legion"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="legion"

# Color support: only when stdout is a terminal.
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  BLUE='\033[0;34m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  BOLD=''
  RESET=''
fi

info() {
  printf "${BLUE}info${RESET}: %s\n" "$1" >&2
}

warn() {
  printf "${YELLOW}warn${RESET}: %s\n" "$1" >&2
}

error() {
  printf "${RED}error${RESET}: %s\n" "$1" >&2
  exit 1
}

success() {
  printf "${GREEN}success${RESET}: %s\n" "$1" >&2
}

# Temp dir with cleanup trap.
TMPDIR_INSTALL=""
cleanup() {
  if [ -n "${TMPDIR_INSTALL}" ] && [ -d "${TMPDIR_INSTALL}" ]; then
    rm -rf "${TMPDIR_INSTALL}"
  fi
}
trap cleanup EXIT INT TERM

# Detect HTTP client: prefer curl, fall back to wget.
FETCH=""
detect_http_client() {
  if [ -n "${FETCH}" ]; then return; fi
  if command -v curl >/dev/null 2>&1; then
    FETCH="curl"
  elif command -v wget >/dev/null 2>&1; then
    FETCH="wget"
  else
    error "Neither curl nor wget found. Please install one and retry."
  fi
}

fetch() {
  local url="$1"
  local output="$2"
  detect_http_client
  if [ "${FETCH}" = "curl" ]; then
    curl -fsSL -o "${output}" "${url}"
  else
    wget -q -O "${output}" "${url}"
  fi
}

# Follow a redirect to get the final URL (used for version resolution).
fetch_redirect_url() {
  local url="$1"
  detect_http_client
  if [ "${FETCH}" = "curl" ]; then
    curl -fsSL -o /dev/null -w '%{url_effective}' "${url}"
  else
    wget --spider -S -O /dev/null "${url}" 2>&1 | grep -i 'Location:' | tail -1 | awk '{print $2}' | tr -d '\r'
  fi
}

# Detect platform.
detect_platform() {
  local os
  os="$(uname -s)"
  case "${os}" in
    Linux)  echo "linux" ;;
    Darwin) echo "macos" ;;
    *)      error "Unsupported operating system: ${os}. Legion supports Linux and macOS." ;;
  esac
}

# Detect architecture, with Rosetta 2 detection on macOS.
detect_arch() {
  local arch platform
  arch="$(uname -m)"
  platform="$1"

  # On macOS, check for Rosetta 2 translation. If the process is translated,
  # the real hardware is arm64, so prefer the native build.
  if [ "${platform}" = "macos" ]; then
    local translated
    translated="$(sysctl -n sysctl.proc_translated 2>/dev/null || echo "0")"
    if [ "${translated}" = "1" ]; then
      warn "Rosetta 2 translation detected. Installing native arm64 binary."
      echo "arm64"
      return
    fi
  fi

  case "${arch}" in
    x86_64|amd64)   echo "x64" ;;
    arm64|aarch64)   echo "arm64" ;;
    *)               error "Unsupported architecture: ${arch}." ;;
  esac
}

# Resolve the version tag to install.
resolve_version() {
  # Priority: CLI argument, then env var, then latest release.
  local version="${1:-${LEGION_VERSION:-}}"
  if [ -n "${version}" ]; then
    # Ensure the version starts with 'v'.
    case "${version}" in
      v*) echo "${version}" ;;
      *)  echo "v${version}" ;;
    esac
    return
  fi

  info "Resolving latest version from GitHub..."
  local redirect_url
  redirect_url="$(fetch_redirect_url "https://github.com/${REPO}/releases/latest")"
  if [ -z "${redirect_url}" ]; then
    error "Could not resolve latest version. Specify a version with: bash -s v0.1.0"
  fi

  # The redirect URL ends with the tag, e.g. .../releases/tag/v0.1.0
  local tag
  tag="${redirect_url##*/}"
  case "${tag}" in
    v[0-9]*) ;;
    *) error "No releases found. Specify a version with: bash -s v0.1.0" ;;
  esac
  echo "${tag}"
}

# Verify checksum of the downloaded archive.
verify_checksum() {
  local archive="$1"
  local checksums_file="$2"
  local platform="$3"
  local archive_name
  archive_name="$(basename "${archive}")"

  info "Verifying checksum..."

  # Extract the expected line from checksums.txt.
  local expected_line
  expected_line="$(grep -F "${archive_name}" "${checksums_file}" || true)"
  if [ -z "${expected_line}" ]; then
    error "No checksum found for ${archive_name} in checksums.txt."
  fi

  # Write a single-line checksum file for verification.
  local check_file="${TMPDIR_INSTALL}/check.txt"
  echo "${expected_line}" > "${check_file}"

  # Use the platform-appropriate checksum tool.
  local checksum_ok=true
  if [ "${platform}" = "macos" ]; then
    (cd "${TMPDIR_INSTALL}" && shasum -a 256 -c "${check_file}" >/dev/null 2>&1) || checksum_ok=false
  else
    (cd "${TMPDIR_INSTALL}" && sha256sum -c "${check_file}" >/dev/null 2>&1) || checksum_ok=false
  fi

  if [ "${checksum_ok}" = "false" ]; then
    error "Checksum verification failed for ${archive_name}. The download may be corrupted."
  fi
  success "Checksum verified."
}

# Add ~/.local/bin to PATH in the appropriate shell config (idempotent).
ensure_path() {
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) return ;;
  esac

  local shell_name
  shell_name="$(basename "${SHELL:-bash}")"
  local path_line=""
  local config_file=""

  case "${shell_name}" in
    bash) config_file="${HOME}/.bashrc" ;;
    zsh)  config_file="${HOME}/.zshrc" ;;
    fish)
      config_file="${HOME}/.config/fish/config.fish"
      path_line='fish_add_path ~/.local/bin'
      ;;
    *)
      warn "${INSTALL_DIR} is not in your PATH. Add it manually to your shell config."
      return
      ;;
  esac

  # Default for bash/zsh (intentionally literal; expands at shell startup).
  # shellcheck disable=SC2016
  [ -z "${path_line}" ] && path_line='export PATH="${HOME}/.local/bin:${PATH}"'

  # Only append if the line is not already present.
  if [ -f "${config_file}" ] && grep -qF "${path_line}" "${config_file}" 2>/dev/null; then
    return
  fi

  info "Adding ${INSTALL_DIR} to PATH in ${config_file}..."
  if mkdir -p "$(dirname "${config_file}")" 2>/dev/null && \
     printf '\n# Added by Legion installer\n%s\n' "${path_line}" >> "${config_file}" 2>/dev/null; then
    warn "Restart your shell or run:  source ${config_file}"
  else
    warn "Could not update ${config_file}. Add ${INSTALL_DIR} to your PATH manually."
  fi
}

# Main installation flow.
main() {
  local version="${1:-}"

  printf '%b\n\n' "${BOLD}Legion Installer${RESET}" >&2

  local platform arch artifact_name
  platform="$(detect_platform)"
  arch="$(detect_arch "${platform}")"
  artifact_name="${BINARY_NAME}-${platform}-${arch}"

  info "Detected platform: ${platform}-${arch}"

  version="$(resolve_version "${version}")"
  info "Installing Legion ${version}..."

  # Create temp directory.
  TMPDIR_INSTALL="$(mktemp -d)"

  local base_url="https://github.com/${REPO}/releases/download/${version}"
  local archive_file="${artifact_name}.tar.gz"
  local archive_path="${TMPDIR_INSTALL}/${archive_file}"
  local checksums_path="${TMPDIR_INSTALL}/checksums.txt"

  # Download archive and checksums.
  info "Downloading ${archive_file}..."
  fetch "${base_url}/${archive_file}" "${archive_path}"
  fetch "${base_url}/checksums.txt" "${checksums_path}"

  # Verify integrity.
  verify_checksum "${archive_path}" "${checksums_path}" "${platform}"

  # Extract and install.
  info "Installing to ${INSTALL_DIR}..."
  if ! mkdir -p "${INSTALL_DIR}" 2>/dev/null; then
    error "Cannot create ${INSTALL_DIR}. Check your permissions."
  fi
  tar xzf "${archive_path}" -C "${TMPDIR_INSTALL}"
  if ! mv "${TMPDIR_INSTALL}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"; then
    error "Cannot write to ${INSTALL_DIR}/${BINARY_NAME}. Check your permissions."
  fi
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  # PATH integration.
  ensure_path

  # Configure Claude Code hooks via legion init.
  info "Configuring Claude Code hooks..."
  if "${INSTALL_DIR}/${BINARY_NAME}" init --force 2>&1; then
    true
  else
    warn "Hook configuration failed. Run 'legion init' manually after installation."
  fi

  # Verify the installed binary runs.
  local installed_version
  installed_version="$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>/dev/null || true)"
  if [ -n "${installed_version}" ]; then
    success "Installed ${installed_version} to ${INSTALL_DIR}/${BINARY_NAME}"
  else
    success "Installed Legion ${version} to ${INSTALL_DIR}/${BINARY_NAME}"
  fi
}

main "$@"
