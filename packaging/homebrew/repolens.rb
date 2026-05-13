# frozen_string_literal: true

# Homebrew formula for RepoLens
# Install: brew install repolens
# Update: brew upgrade repolens

class Repolens < Formula
  desc "CLI tool to audit repositories for best practices, security, and compliance"
  homepage "https://github.com/systm-d/repolens"
  url "https://github.com/systm-d/repolens/archive/refs/tags/v__VERSION__.tar.gz"
  sha256 "__SHA256__"
  license "MIT"
  head "https://github.com/systm-d/repolens.git", branch: "main"

  # Bottles are pre-compiled binaries for faster installation
  # These will be populated by the CI/CD pipeline during release
  # bottle do
  #   sha256 cellar: :any_skip_relocation, arm64_sonoma: "__SHA256_ARM64_SONOMA__"
  #   sha256 cellar: :any_skip_relocation, arm64_ventura: "__SHA256_ARM64_VENTURA__"
  #   sha256 cellar: :any_skip_relocation, sonoma: "__SHA256_SONOMA__"
  #   sha256 cellar: :any_skip_relocation, ventura: "__SHA256_VENTURA__"
  #   sha256 cellar: :any_skip_relocation, x86_64_linux: "__SHA256_LINUX__"
  # end

  depends_on "rust" => :build
  depends_on "git"

  # OpenSSL is required for HTTPS support in reqwest
  uses_from_macos "openssl"

  def install
    # Ensure Cargo uses the correct paths
    ENV["CARGO_HOME"] = buildpath/"cargo"

    # Build with release optimizations
    system "cargo", "install", *std_cargo_args

    # Generate shell completions
    generate_completions_from_executable(bin/"repolens", "completions")

    # Generate and install man page
    system bin/"repolens", "generate-man", "--output", buildpath
    man1.install "repolens.1"
  end

  def caveats
    <<~EOS
      RepoLens requires the GitHub CLI (gh) for some features.
      Install it with: brew install gh

      To initialize RepoLens in a repository:
        cd /path/to/your/repo
        repolens init

      For more information, visit:
        https://github.com/systm-d/repolens
    EOS
  end

  test do
    # Test version output
    assert_match "repolens #{version}", shell_output("#{bin}/repolens --version")

    # Test help output
    assert_match "audit repositories", shell_output("#{bin}/repolens --help")

    # Test init in a temporary directory (dry run)
    system "git", "init", testpath/"test_repo"
    Dir.chdir(testpath/"test_repo") do
      # Just verify the binary runs without error
      system "#{bin}/repolens", "--help"
    end
  end
end
