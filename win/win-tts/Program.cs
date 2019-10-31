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
            var stream = new MemoryStream();
            var format = new SpeechAudioFormatInfo(16000, AudioBitsPerSample.Sixteen, AudioChannel.Mono);
            synth.SetOutputToAudioStream(stream, format);
            synth.SpeakSsml(args[0]);

            var stdout = Console.OpenStandardOutput();
            stream.Position = 0;
            stream.CopyTo(stdout);
        }
    }
}
