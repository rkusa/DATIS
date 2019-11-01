using System;
using System.IO;
using System.Speech.AudioFormat;
using System.Speech.Synthesis;

namespace win_tts
{
    class Program
    {
        static void Main(string[] args)
        {
            var synth = new SpeechSynthesizer();
            switch (args.Length)
            {
                case 0:
                    EnumerateAvailableVoices(synth);
                    break;
                case 1:
                    TextToSpeech(synth, args[0]);
                    break;
                case 2:
                    SelectVoice(synth, args[1]);
                    TextToSpeech(synth, args[0]);
                    break;
                default:
                    Console.Write(@"Usage:
                    win-tts.exe
                        Returns all available voices on STDOUT.

                    win-tts.exe <ssml>
                        Generates audio of the provided SSML and outputs it as WAV stream on STDOUT.

                    win-tts.exe <ssml> <voice>
                        Generates audio of the provided SSML with the given voice name and outputs it as WAV stream on STDOUT.
                        If the voice is not available on the system, the standard voice will be used.
                        A warning will be output on STDERR in this case.");
                    break;
            }
        }

        private static void TextToSpeech(SpeechSynthesizer synth, string text)
        {
            var stream = new MemoryStream();
            var format = new SpeechAudioFormatInfo(16000, AudioBitsPerSample.Sixteen, AudioChannel.Mono);
            synth.SetOutputToAudioStream(stream, format);
            synth.SpeakSsml(text);

            var stdout = Console.OpenStandardOutput();
            stream.Position = 0;
            stream.CopyTo(stdout);
        }

        private static void SelectVoice(SpeechSynthesizer synth, string voice)
        {
            try
            {
                synth.SelectVoice(voice);
            }
            catch (ArgumentException ex)
            {
                Console.Error.WriteLine(ex.Message);
            }
        }

        private static void EnumerateAvailableVoices(SpeechSynthesizer synth)
        {
            foreach (var v in synth.GetInstalledVoices())
                Console.WriteLine(v.VoiceInfo.Name);
        }
    }
}
