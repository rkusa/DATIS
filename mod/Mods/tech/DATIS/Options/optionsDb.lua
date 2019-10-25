local DbOption = require("Options.DbOption")

return {
  gcloudAccessKey = DbOption.new():setValue(""):editbox(),
  awsAccessKey = DbOption.new():setValue(""):editbox(),
  awsPrivateKey = DbOption.new():setValue(""):editbox(),
  -- see https://docs.aws.amazon.com/general/latest/gr/rande.html#pol_region for regions polly is available in
  awsRegion = DbOption.new():setValue("UsEast1"):combo({
    DbOption.Item(_('ap-northeast-1')):Value('ap-northeast-1'),
    DbOption.Item(_('ap-northeast-2')):Value('ap-northeast-2'),
    DbOption.Item(_('ap-south-1')):Value('ap-south-1'),
    DbOption.Item(_('ap-southeast-1')):Value('ap-southeast-1'),
    DbOption.Item(_('ap-southeast-2')):Value('ap-southeast-2'),
    DbOption.Item(_('ca-central-1')):Value('ca-central-1'),
    DbOption.Item(_('cn-northwest-1')):Value('cn-northwest-1'),
    DbOption.Item(_('eu-central-1')):Value('eu-central-1'),
    DbOption.Item(_('eu-north-1')):Value('eu-north-1'),
    DbOption.Item(_('eu-west-1')):Value('eu-west-1'),
    DbOption.Item(_('eu-west-2')):Value('eu-west-2'),
    DbOption.Item(_('eu-west-3')):Value('eu-west-3'),
    DbOption.Item(_('sa-east-1')):Value('sa-east-1'),
    DbOption.Item(_('us-east-1')):Value('us-east-1'),
    DbOption.Item(_('us-east-2')):Value('us-east-2'),
    DbOption.Item(_('us-gov-west-1')):Value('us-gov-west-1'),
    DbOption.Item(_('us-west-1')):Value('us-west-1'),
    DbOption.Item(_('us-west-2')):Value('us-west-2'),
  }),
  srsPort = DbOption.new():setValue("5002"):editbox(),
  debugLoggingEnabled = DbOption.new():setValue(false):checkbox()
}
