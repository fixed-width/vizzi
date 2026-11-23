vim.api.nvim_create_user_command('VizziOpen', function(command)
  if vim.fn.has('nvim-0.10') == 0 then
    vim.notify('Vizzi requires Neovim 0.10 or newer', vim.log.levels.ERROR)
    return
  end
  require('vizzi').open(command.fargs[1])
end, { nargs = '?', complete = 'dir', desc = 'Open the Vizzi workspace in the browser' })

vim.api.nvim_create_user_command('VizziStop', function()
  local companion = package.loaded.vizzi
  if companion then
    companion.stop()
  end
end, { desc = 'Stop the Vizzi workspace' })
