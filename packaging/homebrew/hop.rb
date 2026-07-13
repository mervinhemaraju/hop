# Formula for the mervinhemaraju/homebrew-tap repository (Formula/hop.rb).
# The three sha256 placeholders come from the SHA256SUMS asset attached to
# the GitHub release by the release workflow.
class Hop < Formula
  desc "Fast, interactive context switching for Google Cloud Platform"
  homepage "https://github.com/mervinhemaraju/hop"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/hop-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "2c7b149857aff5e4c908e671f3ab57b96a7472ac24efbd38d11d4214d08359c5"
    end
    on_intel do
      url "https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/hop-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "d832a0094f7a9bcc967472a28a3f43a53116bd8f1c0dc91a32507598805afd4b"
    end
  end

  on_linux do
    url "https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/hop-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
    sha256 "e1cc985847097fabeacdcbc25a1c8ff2911840bfebae4d8745134060038a9845"
  end

  def install
    bin.install "hop"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hop --version")
  end
end
