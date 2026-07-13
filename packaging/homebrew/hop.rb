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
      sha256 "REPLACE_WITH_SHA256_OF_aarch64-apple-darwin"
    end
    on_intel do
      url "https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/hop-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_x86_64-apple-darwin"
    end
  end

  on_linux do
    url "https://github.com/mervinhemaraju/hop/releases/download/v0.1.0/hop-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
    sha256 "REPLACE_WITH_SHA256_OF_x86_64-unknown-linux-musl"
  end

  def install
    bin.install "hop"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hop --version")
  end
end
