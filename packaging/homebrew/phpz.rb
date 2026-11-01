class Phpz < Formula
  desc "PHP version manager with per-project versions and extensions"
  homepage "https://github.com/CaduVarela/phpz"
  version "0.5.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/CaduVarela/phpz/releases/download/v0.5.0/phpz-macos-arm64"
      sha256 "810199afd96f9d3e928512ea544d3ea3815439ac6bc8fcb0fbb35c5df7ffe795"
    end
    on_intel do
      url "https://github.com/CaduVarela/phpz/releases/download/v0.5.0/phpz-macos-x64"
      sha256 "43f6c1e3f14924a933b306b0eabaca8291f18a7df4f555e2d704921f4507819a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/CaduVarela/phpz/releases/download/v0.5.0/phpz-linux-arm64"
      sha256 "d92565a84eb8764a22abb08bab25b609aba20783ef2f92ba1f0fed75d7ca8477"
    end
    on_intel do
      url "https://github.com/CaduVarela/phpz/releases/download/v0.5.0/phpz-linux-x64"
      sha256 "896d2258dc21ba9c5db602450b252b9e47de3eb526eedcdb9e58d2f5feb484ed"
    end
  end

  def install
    bin.install Dir["phpz-*"].first => "phpz"
  end

  def caveats
    "Run `phpz setup` once to put the php shim on your PATH."
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/phpz --version")
  end
end
