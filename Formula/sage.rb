# Homebrew formula for Sage Agent
# https://github.com/majiayu000/sage
#
# Installation:
#   brew tap majiayu000/sage
#   brew install sage
#
# Or direct install:
#   brew install majiayu000/sage/sage

class Sage < Formula
  desc "Blazing fast code agent in pure Rust"
  homepage "https://github.com/majiayu000/sage"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/majiayu000/sage/releases/download/v#{version}/sage-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"

      def install
        bin.install "sage"
      end
    end

    on_intel do
      url "https://github.com/majiayu000/sage/releases/download/v#{version}/sage-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_X64"

      def install
        bin.install "sage"
      end
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/majiayu000/sage/releases/download/v#{version}/sage-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"

      def install
        bin.install "sage"
      end
    end

    on_intel do
      url "https://github.com/majiayu000/sage/releases/download/v#{version}/sage-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X64"

      def install
        bin.install "sage"
      end
    end
  end

  def caveats
    <<~EOS
      Sage requires an LLM API key to function. Configure it with:

        # For Anthropic Claude (recommended)
        export ANTHROPIC_API_KEY="your-api-key"

        # For OpenAI
        export OPENAI_API_KEY="your-api-key"

        # For local models with Ollama
        # No API key needed, just run: ollama serve

      Quick start:
        sage interactive         # Start interactive mode
        sage "Your task here"    # Run a one-shot task

      Documentation: https://github.com/majiayu000/sage
    EOS
  end

  test do
    assert_match "sage", shell_output("#{bin}/sage --version")
  end
end
