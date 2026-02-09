class Meepo < Formula
  desc "Local AI agent — connects Claude to your email, calendar, and more"
  homepage "https://github.com/kavymi/meepo"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/kavymi/meepo/releases/download/v#{version}/meepo-darwin-arm64.tar.gz"
      sha256 "fa8004e94e33c65661cd29f2e7103a0e8db56484b39f355601e18c576597bb0a"
    else
      url "https://github.com/kavymi/meepo/releases/download/v#{version}/meepo-darwin-x64.tar.gz"
      sha256 "64bdb9e61af071139731d83554bcf9e93f36b9e27c76f8c72fae46af6ba7809c"
    end
  end

  def install
    bin.install "meepo"
  end

  def caveats
    <<~EOS
      Run the setup wizard to configure API keys:
        meepo setup

      Then start the agent:
        meepo start

      Enable channels (Discord, Slack, iMessage) in:
        ~/.meepo/config.toml
    EOS
  end

  test do
    assert_match "Meepo", shell_output("#{bin}/meepo --version")
  end
end
