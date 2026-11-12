# Ruby LSP's private server bootstrap; project add-ons are source, not extensions.
require "ruby_lsp/internal"

module VizziReadOnlyRubyLsp
  def load_addons(...)
  end
end

RubyLsp::Server.prepend(VizziReadOnlyRubyLsp)
