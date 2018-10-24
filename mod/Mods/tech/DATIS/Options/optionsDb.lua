local DbOption = require('Options.DbOption')

return {
    gcloudAccessKey = DbOption.new():setValue(''):editbox(),
}
