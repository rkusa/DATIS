# Settings

## Location

`Saved Games\DCS.openbeta\Config\DATIS.json`

## Example

```json
{
  "default_voice": "WIN",
  "gcloud": {
    "key": "YOUR_KEY"
  },
  "aws": {
    "key": "YOUR_KEY",
    "secret": "YOUR_SECRET",
    "region": "eu-central-1"
  },
  "srs_port": 5002,
  "debug": false
}
```

## Available Settings

<table>

<tr>
<td valign="top">`glcoud.key`</td>
<td valign="top">

Your Google cloud access key (Go to https://console.cloud.google.com/apis/credentials and create an API key and restrict API access to Google Text-to-Speech)

</td>
</tr>

<tr>
<td valign="top">`aws.key`</td>
<td valign="top" rowspan="2">

Your AWS access key and secret (Go to https://console.aws.amazon.com/iam/home#/users and create a new user with `AmazonPollyReadOnlyAccess` permissions)

</td>
</tr>

<tr>
<td valign="top">`aws.secret`</td>
</tr>

<tr>
<td valign="top">`aws.region`</td>
<td valign="top">

Your AWS region (see https://docs.aws.amazon.com/general/latest/gr/rande.html#pol_region for regions polly is available in), possible values are: `ap-northeast-1`, `ap-northeast-2`, `ap-south-1`, `ap-southeast-1`, `ap-southeast-2`, `ca-central-1`, `cn-northwest-1`, `eu-central-1`, `eu-north-1`, `eu-west-1`, `eu-west-2`, `eu-west-3`, `sa-east-1`, `us-east-1`, `us-east-2`, `us-gov-west-1`, `us-west-1`, `us-west-2`

</td>
</tr>

<tr>
<td valign="top">`srsPort`</td>
<td valign="top">

The port of your locally running SRS server (default: `5002`)

</td>
</tr>

<tr>
<td valign="top">`debug`</td>
<td valign="top">

Whether debug logging is enabled or not (default: `false`)

</td>
</tr>
<tr>
<td valign="top">`default_voice`</td>
<td valign="top">

  <table>

  <tr><th>Value</th><th></th></tr>
  <tr><td>WIN</td><td>Windows TTS: System default voice</td></tr>
  <tr><td>WIN:Catherine</td><td>Windows TTS: Catherine (en-AU)</td></tr>
  <tr><td>WIN:James</td><td>Windows TTS: James (en-AU)</td></tr>
  <tr><td>WIN:Linda</td><td>Windows TTS: Linda (en-CA)</td></tr>
  <tr><td>WIN:Richard</td><td>Windows TTS: Richard (en-CA)</td></tr>
  <tr><td>WIN:George</td><td>Windows TTS: George (en-GB)</td></tr>
  <tr><td>WIN:Hazel</td><td>Windows TTS: Hazel (en-GB)</td></tr>
  <tr><td>WIN:Susan</td><td>Windows TTS: Susan (en-GB)</td></tr>
  <tr><td>WIN:Sean</td><td>Windows TTS: Sean (en-IE)</td></tr>
  <tr><td>WIN:Heera</td><td>Windows TTS: Heera (en-IN)</td></tr>
  <tr><td>WIN:Ravi</td><td>Windows TTS: Ravi (en-IN)</td></tr>
  <tr><td>WIN:David</td><td>Windows TTS: David (en-US)</td></tr>
  <tr><td>WIN:Zira</td><td>Windows TTS: Zira (en-US)</td></tr>
  <tr><td>WIN:Mark</td><td>Windows TTS: Mark (en-US)</td></tr>
  <tr><td>GC:en-AU-Standard-A</td><td>GCloud: en-AU-Standard-A</td></tr>
  <tr><td>GC:en-AU-Standard-B</td><td>GCloud: en-AU-Standard-B</td></tr>
  <tr><td>GC:en-AU-Standard-C</td><td>GCloud: en-AU-Standard-C</td></tr>
  <tr><td>GC:en-AU-Standard-D</td><td>GCloud: en-AU-Standard-D</td></tr>
  <tr><td>GC:en-AU-Wavenet-A</td><td>GCloud: en-AU-Wavenet-A</td></tr>
  <tr><td>GC:en-AU-Wavenet-B</td><td>GCloud: en-AU-Wavenet-B</td></tr>
  <tr><td>GC:en-AU-Wavenet-C</td><td>GCloud: en-AU-Wavenet-C</td></tr>
  <tr><td>GC:en-AU-Wavenet-D</td><td>GCloud: en-AU-Wavenet-D</td></tr>
  <tr><td>GC:en-IN-Standard-A</td><td>GCloud: en-IN-Standard-A</td></tr>
  <tr><td>GC:en-IN-Standard-B</td><td>GCloud: en-IN-Standard-B</td></tr>
  <tr><td>GC:en-IN-Standard-C</td><td>GCloud: en-IN-Standard-C</td></tr>
  <tr><td>GC:en-IN-Standard-D</td><td>GCloud: en-IN-Standard-D</td></tr>
  <tr><td>GC:en-IN-Wavenet-A</td><td>GCloud: en-IN-Wavenet-A</td></tr>
  <tr><td>GC:en-IN-Wavenet-B</td><td>GCloud: en-IN-Wavenet-B</td></tr>
  <tr><td>GC:en-IN-Wavenet-C</td><td>GCloud: en-IN-Wavenet-C</td></tr>
  <tr><td>GC:en-IN-Wavenet-D</td><td>GCloud: en-IN-Wavenet-D</td></tr>
  <tr><td>GC:en-GB-Standard-A</td><td>GCloud: en-GB-Standard-A</td></tr>
  <tr><td>GC:en-GB-Standard-B</td><td>GCloud: en-GB-Standard-B</td></tr>
  <tr><td>GC:en-GB-Standard-C</td><td>GCloud: en-GB-Standard-C</td></tr>
  <tr><td>GC:en-GB-Standard-D</td><td>GCloud: en-GB-Standard-D</td></tr>
  <tr><td>GC:en-GB-Standard-F</td><td>GCloud: en-GB-Standard-F</td></tr>
  <tr><td>GC:en-GB-Wavenet-A</td><td>GCloud: en-GB-Wavenet-A</td></tr>
  <tr><td>GC:en-GB-Wavenet-B</td><td>GCloud: en-GB-Wavenet-B</td></tr>
  <tr><td>GC:en-GB-Wavenet-C</td><td>GCloud: en-GB-Wavenet-C</td></tr>
  <tr><td>GC:en-GB-Wavenet-D</td><td>GCloud: en-GB-Wavenet-D</td></tr>
  <tr><td>GC:en-GB-Wavenet-F</td><td>GCloud: en-GB-Wavenet-F</td></tr>
  <tr><td>GC:en-US-Standard-B</td><td>GCloud: en-US-Standard-B</td></tr>
  <tr><td>GC:en-US-Standard-C</td><td>GCloud: en-US-Standard-C</td></tr>
  <tr><td>GC:en-US-Standard-D</td><td>GCloud: en-US-Standard-D</td></tr>
  <tr><td>GC:en-US-Standard-E</td><td>GCloud: en-US-Standard-E</td></tr>
  <tr><td>GC:en-US-Standard-G</td><td>GCloud: en-US-Standard-G</td></tr>
  <tr><td>GC:en-US-Standard-H</td><td>GCloud: en-US-Standard-H</td></tr>
  <tr><td>GC:en-US-Standard-I</td><td>GCloud: en-US-Standard-I</td></tr>
  <tr><td>GC:en-US-Standard-J</td><td>GCloud: en-US-Standard-J</td></tr>
  <tr><td>GC:en-US-Wavenet-A</td><td>GCloud: en-US-Wavenet-A</td></tr>
  <tr><td>GC:en-US-Wavenet-B</td><td>GCloud: en-US-Wavenet-B</td></tr>
  <tr><td>GC:en-US-Wavenet-C</td><td>GCloud: en-US-Wavenet-C</td></tr>
  <tr><td>GC:en-US-Wavenet-D</td><td>GCloud: en-US-Wavenet-D</td></tr>
  <tr><td>GC:en-US-Wavenet-E</td><td>GCloud: en-US-Wavenet-E</td></tr>
  <tr><td>GC:en-US-Wavenet-F</td><td>GCloud: en-US-Wavenet-F</td></tr>
  <tr><td>GC:en-US-Wavenet-G</td><td>GCloud: en-US-Wavenet-G</td></tr>
  <tr><td>GC:en-US-Wavenet-H</td><td>GCloud: en-US-Wavenet-H</td></tr>
  <tr><td>GC:en-US-Wavenet-I</td><td>GCloud: en-US-Wavenet-I</td></tr>
  <tr><td>GC:en-US-Wavenet-J</td><td>GCloud: en-US-Wavenet-J</td></tr>
  <tr><td>AWS:Nicole</td><td>AWS: Nicole (en-AU)</td></tr>
  <tr><td>AWS:Olivia</td><td>AWS: Olivia (en-AU)</td></tr>
  <tr><td>AWS:Russell</td><td>AWS: Russell (en-AU)</td></tr>
  <tr><td>AWS:Amy</td><td>AWS: Amy (en-GB)</td></tr>
  <tr><td>AWS:Emma</td><td>AWS: Emma (en-GB)</td></tr>
  <tr><td>AWS:Brian</td><td>AWS: Brian (en-GB)</td></tr>
  <tr><td>AWS:Aditi</td><td>AWS: Aditi (en-IN)</td></tr>
  <tr><td>AWS:Raveena</td><td>AWS: Raveena (en-IN)</td></tr>
  <tr><td>AWS:Ivy</td><td>AWS: Ivy (en-US)</td></tr>
  <tr><td>AWS:Joanna</td><td>AWS: Joanna (en-US)</td></tr>
  <tr><td>AWS:Kendra</td><td>AWS: Kendra (en-US)</td></tr>
  <tr><td>AWS:Kimberly</td><td>AWS: Kimberly (en-US)</td></tr>
  <tr><td>AWS:Salli</td><td>AWS: Salli (en-US)</td></tr>
  <tr><td>AWS:Joey</td><td>AWS: Joey (en-US)</td></tr>
  <tr><td>AWS:Justin</td><td>AWS: Justin (en-US)</td></tr>
  <tr><td>AWS:Kevin</td><td>AWS: Kevin (en-US)</td></tr>
  <tr><td>AWS:Matthew</td><td>AWS: Matthew (en-US)</td></tr>
  <tr><td>AWS:Geraint</td><td>AWS: Geraint (en-US)</td></tr>

  </table>

</td>
</tr>

</table>
