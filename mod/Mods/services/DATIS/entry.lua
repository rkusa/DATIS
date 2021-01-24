declare_plugin("DATIS", {
  installed     = true,
  dirName       = current_mod_path,
  binaries      = {
    "datis.dll",
  },

  version       = "2.2.1",
  state         = "installed",
  developerName = "github.com/rkusa",
  info          = _("DATIS enables a DCS server with an SRS server running on the same machine (TCP=127.0.0.1) to get weather from the mission for stations and frequencies set in the mission editor, and then to report same in a standardized format over SRS using either the Amazon or Google text to speech engines."),

  Skins = {
    {
      name = "DATIS",
      dir  = "Theme"
    },
  },

  Options = {
    {
      name   = "DATIS",
      nameId = "DATIS",
      dir    = "Options",
    },
  },
})

plugin_done()
