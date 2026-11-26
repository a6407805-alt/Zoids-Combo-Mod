// Zoids Combo Launcher - keeps the patcher up to date, builds the combo game
// from the player's own discs, and launches it in Dolphin.
// No game data is ever downloaded: only the patcher and this launcher.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Reflection;
using System.Text;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Microsoft.Win32;

namespace ZoidsComboLauncher
{
    public static class Program
    {
        [STAThread]
        public static void Main()
        {
            var app = new Application();
            app.Run(new MainWindow());
        }
    }

    static class Cfg
    {
        public const string Repo = "a6407805-alt/Zoids-Combo-Mod";
        public const string PatcherAsset = "ZoidsComboPatcher.exe";
        public const string LauncherAsset = "ZoidsComboLauncher.exe";
        public const string Vs3Id = "GZVJDA", BlId = "GZSE70";
        public static readonly string DataDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "ZoidsCombo");
        public static readonly string GameDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "Zoids Combo");
        public static string PatcherPath => Path.Combine(DataDir, PatcherAsset);
        public static string OutIso => Path.Combine(GameDir, "Zoids Combo.iso");
        static string SettingsFile => Path.Combine(DataDir, "settings.txt");

        static Dictionary<string, string> s;
        public static string Get(string k) { Load(); return s.TryGetValue(k, out var v) ? v : ""; }
        public static void Set(string k, string v)
        {
            Load(); s[k] = v ?? "";
            Directory.CreateDirectory(DataDir);
            File.WriteAllLines(SettingsFile, s.Select(p => p.Key + "=" + p.Value));
        }
        static void Load()
        {
            if (s != null) return;
            s = new Dictionary<string, string>();
            try
            {
                foreach (var l in File.ReadAllLines(SettingsFile))
                {
                    int i = l.IndexOf('=');
                    if (i > 0) s[l.Substring(0, i)] = l.Substring(i + 1);
                }
            }
            catch { }
        }
    }

    public class MainWindow : Window
    {
        readonly TextBlock status = new TextBlock();
        readonly TextBox log = new TextBox();
        readonly TextBlock vs3Label = new TextBlock(), blLabel = new TextBlock(), dolLabel = new TextBlock(), verLabel = new TextBlock();
        readonly Button play = new Button(), rebuild = new Button();
        readonly ProgressBar bar = new ProgressBar();
        static readonly Brush Bg = new SolidColorBrush(Color.FromRgb(0x16, 0x1B, 0x26));
        static readonly Brush Panel = new SolidColorBrush(Color.FromRgb(0x22, 0x2A, 0x3A));
        static readonly Brush Accent = new SolidColorBrush(Color.FromRgb(0xC8, 0x10, 0x10));
        static readonly Brush Fg = Brushes.WhiteSmoke;
        bool busy;

        static Version MyVersion => Assembly.GetExecutingAssembly().GetName().Version;

        public MainWindow()
        {
            Title = "Zoids Combo Launcher";
            Width = 640; Height = 560; Background = Bg; WindowStartupLocation = WindowStartupLocation.CenterScreen;
            ResizeMode = ResizeMode.CanMinimize;

            var root = new StackPanel { Margin = new Thickness(18) };
            root.Children.Add(new TextBlock { Text = "ZOIDS COMBO", FontSize = 30, FontWeight = FontWeights.Black, Foreground = Fg });
            root.Children.Add(new TextBlock { Text = "Vs. III + Battle Legends", FontSize = 15, Foreground = Accent, Margin = new Thickness(0, 0, 0, 12) });

            var files = new StackPanel { Background = Panel };
            files.Children.Add(Row("Zoids Vs. III (T-En) ISO", vs3Label, () => Pick("vs3", Cfg.Vs3Id, "Zoids Vs. III (T-En)")));
            files.Children.Add(Row("Battle Legends ISO", blLabel, () => Pick("bl", Cfg.BlId, "Zoids: Battle Legends")));
            files.Children.Add(Row("Dolphin.exe", dolLabel, PickDolphin));
            root.Children.Add(files);

            status.Foreground = Fg; status.FontSize = 14; status.Margin = new Thickness(0, 12, 0, 6); status.TextWrapping = TextWrapping.Wrap;
            root.Children.Add(status);
            bar.Height = 6; bar.Foreground = Accent; bar.Background = Panel; bar.BorderThickness = new Thickness(0); bar.IsIndeterminate = false;
            root.Children.Add(bar);

            var buttons = new WrapPanel { Margin = new Thickness(0, 12, 0, 8) };
            StyleButton(play, "▶  PLAY", 180, true); play.Click += async (a, b) => await PlayAsync();
            StyleButton(rebuild, "Rebuild game", 120); rebuild.Click += async (a, b) => await BuildAsync(true);
            var online = new Button(); StyleButton(online, "Play online…", 110); online.Click += (a, b) => OnlineHelp();
            var folder = new Button(); StyleButton(folder, "Game folder", 105);
            folder.Click += (a, b) => { Directory.CreateDirectory(Cfg.GameDir); Process.Start("explorer.exe", "\"" + Cfg.GameDir + "\""); };
            buttons.Children.Add(play); buttons.Children.Add(rebuild); buttons.Children.Add(online); buttons.Children.Add(folder);
            root.Children.Add(buttons);

            log.Height = 150; log.IsReadOnly = true; log.Background = Panel; log.Foreground = Brushes.LightGray; log.BorderThickness = new Thickness(0);
            log.FontFamily = new FontFamily("Consolas"); log.FontSize = 11; log.VerticalScrollBarVisibility = ScrollBarVisibility.Auto; log.TextWrapping = TextWrapping.Wrap;
            root.Children.Add(log);
            verLabel.Foreground = Brushes.Gray; verLabel.FontSize = 11; verLabel.Margin = new Thickness(0, 6, 0, 0);
            root.Children.Add(verLabel);
            Content = root;

            Loaded += async (a, b) => await StartupAsync();
        }

        static void StyleButton(Button b, string text, double w, bool primary = false)
        {
            b.Content = text; b.MinWidth = w; b.Height = primary ? 44 : 34; b.Margin = new Thickness(0, 0, 8, 6);
            b.FontSize = primary ? 18 : 13; b.FontWeight = primary ? FontWeights.Bold : FontWeights.Normal;
            b.Background = primary ? Accent : Panel; b.Foreground = Fg; b.BorderThickness = new Thickness(0); b.Padding = new Thickness(10, 0, 10, 0);
        }

        UIElement Row(string name, TextBlock value, Action change)
        {
            var g = new Grid { Margin = new Thickness(10, 6, 10, 6) };
            g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(170) });
            g.ColumnDefinitions.Add(new ColumnDefinition());
            g.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            var n = new TextBlock { Text = name, Foreground = Brushes.Gray, VerticalAlignment = VerticalAlignment.Center };
            value.Foreground = Fg; value.VerticalAlignment = VerticalAlignment.Center; value.TextTrimming = TextTrimming.CharacterEllipsis;
            var b = new Button(); StyleButton(b, "Change…", 70); b.Height = 26; b.Margin = new Thickness(8, 0, 0, 0); b.Click += (x, y) => change();
            Grid.SetColumn(value, 1); Grid.SetColumn(b, 2);
            g.Children.Add(n); g.Children.Add(value); g.Children.Add(b);
            return g;
        }

        void Log(string t) => Dispatcher.Invoke(() => { log.AppendText(t + "\n"); log.ScrollToEnd(); });
        void Status(string t) => Dispatcher.Invoke(() => status.Text = t);
        void Busy(bool on) => Dispatcher.Invoke(() => { busy = on; bar.IsIndeterminate = on; play.IsEnabled = rebuild.IsEnabled = !on; });

        void Refresh()
        {
            string Show(string p) => string.IsNullOrEmpty(p) ? "(not found — click Change…)" : Path.GetFileName(p);
            vs3Label.Text = Show(Cfg.Get("vs3")); blLabel.Text = Show(Cfg.Get("bl")); dolLabel.Text = Show(Cfg.Get("dolphin"));
            var tag = Cfg.Get("patcherTag");
            verLabel.Text = $"Launcher v{MyVersion.ToString(3)}   ·   Game patch {(tag == "" ? "not installed" : tag)}   ·   Updates from github.com/{Cfg.Repo}";
        }

        // ---------- startup ----------
        async Task StartupAsync()
        {
            Busy(true);
            try
            {
                AutoFind();
                Refresh();
                Status("Checking for updates…");
                await CheckUpdatesAsync();
                Refresh();
                if (NeedsBuild() && HaveDiscs()) await BuildAsync(false, alreadyBusy: true);
                else if (!HaveDiscs()) Status("Pick your two game ISOs above, then press PLAY.");
                else Status("Ready. Press PLAY.");
            }
            catch (Exception e) { Status("Something went wrong: " + e.Message); Log(e.ToString()); }
            finally { Busy(false); Refresh(); }
        }

        bool HaveDiscs() => File.Exists(Cfg.Get("vs3")) && File.Exists(Cfg.Get("bl"));
        bool NeedsBuild() => !File.Exists(Cfg.OutIso) || Cfg.Get("builtTag") != Cfg.Get("patcherTag") ||
                             Cfg.Get("builtFrom") != Cfg.Get("vs3") + "|" + Cfg.Get("bl");

        static string ReadId(string p)
        {
            try { using (var f = File.OpenRead(p)) { var b = new byte[6]; return f.Read(b, 0, 6) == 6 ? Encoding.ASCII.GetString(b) : ""; } }
            catch { return ""; }
        }

        void AutoFind()
        {
            bool needVs3 = !File.Exists(Cfg.Get("vs3")), needBl = !File.Exists(Cfg.Get("bl"));
            if (needVs3 || needBl)
            {
                Status("Looking for your game ISOs…");
                var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
                var roots = new[] { AppDomain.CurrentDomain.BaseDirectory,
                    Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
                    Path.Combine(home, "Downloads"), Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments) };
                foreach (var f in roots.Distinct().SelectMany(r => Scan(r, 3)))
                {
                    if (string.Equals(Path.GetFullPath(f), Path.GetFullPath(Cfg.OutIso), StringComparison.OrdinalIgnoreCase)) continue;
                    var id = ReadId(f);
                    if (needVs3 && id == Cfg.Vs3Id) { Cfg.Set("vs3", f); needVs3 = false; Log("Found Vs. III: " + f); }
                    if (needBl && id == Cfg.BlId) { Cfg.Set("bl", f); needBl = false; Log("Found Battle Legends: " + f); }
                }
            }
            if (!File.Exists(Cfg.Get("dolphin")))
            {
                var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
                var cands = new List<string>();
                foreach (var r in new[] { Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), Path.Combine(home, "Downloads"),
                                          Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"C:\" })
                    cands.AddRange(Scan(r, 2, "Dolphin.exe"));
                var hit = cands.FirstOrDefault();
                if (hit != null) { Cfg.Set("dolphin", hit); Log("Found Dolphin: " + hit); }
            }
        }

        static IEnumerable<string> Scan(string dir, int depth, string exactName = null)
        {
            if (string.IsNullOrEmpty(dir) || !Directory.Exists(dir)) yield break;
            string[] files = new string[0], dirs = new string[0];
            try { files = Directory.GetFiles(dir); dirs = depth > 0 ? Directory.GetDirectories(dir) : dirs; } catch { }
            foreach (var f in files)
            {
                var n = Path.GetFileName(f);
                if (exactName != null ? string.Equals(n, exactName, StringComparison.OrdinalIgnoreCase)
                                      : n.EndsWith(".iso", StringComparison.OrdinalIgnoreCase) || n.EndsWith(".gcm", StringComparison.OrdinalIgnoreCase))
                    yield return f;
            }
            foreach (var d in dirs) foreach (var f in Scan(d, depth - 1, exactName)) yield return f;
        }

        void Pick(string key, string id, string what)
        {
            var dlg = new OpenFileDialog { Title = "Choose your " + what + " ISO", Filter = "GameCube disc (*.iso;*.gcm)|*.iso;*.gcm|All files|*.*" };
            if (dlg.ShowDialog(this) != true) return;
            var got = ReadId(dlg.FileName);
            if (dlg.FileName.EndsWith(".rvz", StringComparison.OrdinalIgnoreCase))
            { MessageBox.Show(this, "That's a compressed RVZ. In Dolphin: right-click the game → Convert File… → Format: ISO, then pick the .iso here.", "Need an ISO"); return; }
            if (got != id) { MessageBox.Show(this, $"That file isn't {what} (game ID {got}, expected {id}).", "Wrong disc"); return; }
            Cfg.Set(key, dlg.FileName); Refresh();
        }

        void PickDolphin()
        {
            var dlg = new OpenFileDialog { Title = "Find Dolphin.exe", Filter = "Dolphin.exe|Dolphin.exe|Programs|*.exe" };
            if (dlg.ShowDialog(this) == true) { Cfg.Set("dolphin", dlg.FileName); Refresh(); }
        }

        // ---------- updates ----------
        static HttpClient Http()
        {
            ServicePointManager.SecurityProtocol |= SecurityProtocolType.Tls12;
            var h = new HttpClient { Timeout = TimeSpan.FromMinutes(10) };
            h.DefaultRequestHeaders.UserAgent.ParseAdd("ZoidsComboLauncher/" + MyVersion);
            return h;
        }

        static Version ParseTag(string tag)
        {
            var t = (tag ?? "").TrimStart('v', 'V').Split('-')[0];
            while (t.Count(c => c == '.') < 2) t += ".0";
            return Version.TryParse(t, out var v) ? v : new Version(0, 0, 0);
        }

        async Task CheckUpdatesAsync()
        {
            Dictionary<string, object> rel;
            try
            {
                using (var h = Http())
                {
                    var json = await h.GetStringAsync($"https://api.github.com/repos/{Cfg.Repo}/releases/latest");
                    rel = new JavaScriptSerializer().Deserialize<Dictionary<string, object>>(json);
                }
            }
            catch (Exception e)
            {
                Log("Couldn't check for updates (" + e.Message + "). Using what's installed.");
                BundledPatcherFallback();
                return;
            }
            var tag = rel["tag_name"] as string;
            var assets = ((System.Collections.ArrayList)rel["assets"]).Cast<Dictionary<string, object>>()
                .ToDictionary(a => (string)a["name"], a => (string)a["browser_download_url"], StringComparer.OrdinalIgnoreCase);
            Log($"Latest release: {tag}");

            // 1) launcher self-update
            if (ParseTag(tag) > MyVersion && assets.TryGetValue(Cfg.LauncherAsset, out var lurl))
            {
                Status($"Updating launcher to {tag}…");
                var me = Assembly.GetExecutingAssembly().Location;
                var tmp = me + ".new";
                using (var h = Http()) File.WriteAllBytes(tmp, await h.GetByteArrayAsync(lurl));
                var bat = Path.Combine(Path.GetTempPath(), "zoidscombo_update.bat");
                File.WriteAllText(bat, "@echo off\r\ntimeout /t 2 /nobreak >nul\r\n" +
                    $"move /y \"{tmp}\" \"{me}\" >nul\r\nstart \"\" \"{me}\"\r\ndel \"%~f0\"\r\n");
                Process.Start(new ProcessStartInfo("cmd.exe", "/c \"" + bat + "\"") { CreateNoWindow = true, UseShellExecute = false });
                Application.Current.Shutdown();
                return;
            }
            // 2) patcher update
            if ((Cfg.Get("patcherTag") != tag || !File.Exists(Cfg.PatcherPath)) && assets.TryGetValue(Cfg.PatcherAsset, out var purl))
            {
                Status($"Downloading game patch {tag}…");
                Directory.CreateDirectory(Cfg.DataDir);
                using (var h = Http()) File.WriteAllBytes(Cfg.PatcherPath + ".tmp", await h.GetByteArrayAsync(purl));
                if (File.Exists(Cfg.PatcherPath)) File.Delete(Cfg.PatcherPath);
                File.Move(Cfg.PatcherPath + ".tmp", Cfg.PatcherPath);
                Cfg.Set("patcherTag", tag);
                Log($"Game patch updated to {tag}.");
                if (rel.TryGetValue("body", out var notes) && notes is string n && n.Trim() != "") Log("What's new:\n" + n.Trim());
            }
            else Log("Game patch is up to date.");
        }

        void BundledPatcherFallback()
        {
            if (File.Exists(Cfg.PatcherPath)) return;
            var bundled = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, Cfg.PatcherAsset);
            if (!File.Exists(bundled)) bundled = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Zoids Combo Patcher.exe");
            if (File.Exists(bundled))
            {
                Directory.CreateDirectory(Cfg.DataDir);
                File.Copy(bundled, Cfg.PatcherPath, true);
                Cfg.Set("patcherTag", "bundled");
                Log("Using the patcher that came with the launcher.");
            }
        }

        // ---------- build + play ----------
        async Task<bool> BuildAsync(bool forced, bool alreadyBusy = false)
        {
            if (!alreadyBusy && busy) return false;
            if (!HaveDiscs()) { Status("Pick both game ISOs first (Change… buttons above)."); return false; }
            if (!File.Exists(Cfg.PatcherPath)) BundledPatcherFallback();
            if (!File.Exists(Cfg.PatcherPath)) { Status("The game patch isn't downloaded yet. Check your internet and reopen the launcher."); return false; }
            if (!alreadyBusy) Busy(true);
            try
            {
                Status("Building Zoids Combo from your discs… (about a minute)");
                Directory.CreateDirectory(Cfg.GameDir);
                var psi = new ProcessStartInfo(Cfg.PatcherPath,
                    $"--vs3 \"{Cfg.Get("vs3")}\" --bl \"{Cfg.Get("bl")}\" --out \"{Cfg.OutIso}\"")
                { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
                var output = new StringBuilder();
                int code = await Task.Run(() =>
                {
                    using (var p = Process.Start(psi))
                    {
                        p.OutputDataReceived += (s, e) => { if (e.Data != null && !e.Data.Trim().EndsWith("%")) { output.AppendLine(e.Data); Log(e.Data); } };
                        p.ErrorDataReceived += (s, e) => { if (e.Data != null) Log(e.Data); };
                        p.BeginOutputReadLine(); p.BeginErrorReadLine(); p.WaitForExit(); return p.ExitCode;
                    }
                });
                if (code != 0) { Status("Build failed — see the log below."); return false; }
                Cfg.Set("builtTag", Cfg.Get("patcherTag"));
                Cfg.Set("builtFrom", Cfg.Get("vs3") + "|" + Cfg.Get("bl"));
                AddToDolphinGameList();
                Status(output.ToString().Contains("Netplay-ready") ? "Game built and Netplay-ready. Press PLAY." : "Game built. Press PLAY.");
                return true;
            }
            finally { if (!alreadyBusy) Busy(false); Refresh(); }
        }

        async Task PlayAsync()
        {
            if (busy) return;
            if (NeedsBuild() && !await BuildAsync(false)) return;
            var dol = Cfg.Get("dolphin");
            if (!File.Exists(dol)) { PickDolphin(); dol = Cfg.Get("dolphin"); if (!File.Exists(dol)) return; }
            Process.Start(new ProcessStartInfo(dol, $"-e \"{Cfg.OutIso}\"") { UseShellExecute = false, WorkingDirectory = Path.GetDirectoryName(dol) });
            Status("Launched in Dolphin. Have fun!");
        }

        // Adds the game folder to Dolphin's game list so it shows up for NetPlay.
        void AddToDolphinGameList()
        {
            try
            {
                var dol = Cfg.Get("dolphin");
                if (!File.Exists(dol)) return;
                if (Process.GetProcessesByName("Dolphin").Length > 0) { Log("Close Dolphin and press Rebuild once to add the game to Dolphin's list automatically."); return; }
                var dir = Path.GetDirectoryName(dol);
                var cands = new List<string>();
                if (File.Exists(Path.Combine(dir, "portable.txt"))) cands.Add(Path.Combine(dir, "User"));
                cands.Add(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Dolphin Emulator"));
                cands.Add(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "Dolphin Emulator"));
                var ini = cands.Select(c => Path.Combine(c, "Config", "Dolphin.ini")).FirstOrDefault(File.Exists);
                if (ini == null) return;
                var lines = File.ReadAllLines(ini).ToList();
                if (lines.Any(l => l.StartsWith("ISOPath", StringComparison.OrdinalIgnoreCase) &&
                                   l.Substring(l.IndexOf('=') + 1).Trim().TrimEnd('\\', '/').Equals(Cfg.GameDir.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))) return;
                int gen = lines.FindIndex(l => l.Trim().Equals("[General]", StringComparison.OrdinalIgnoreCase));
                if (gen < 0) { lines.Add("[General]"); gen = lines.Count - 1; }
                int end = lines.FindIndex(gen + 1, l => l.TrimStart().StartsWith("["));
                if (end < 0) end = lines.Count;
                int countIdx = -1, count = 0;
                for (int i = gen + 1; i < end; i++)
                    if (lines[i].StartsWith("ISOPaths", StringComparison.OrdinalIgnoreCase)) { countIdx = i; int.TryParse(lines[i].Split('=')[1].Trim(), out count); }
                var newLine = $"ISOPath{count} = {Cfg.GameDir}";
                if (countIdx >= 0) { lines[countIdx] = $"ISOPaths = {count + 1}"; lines.Insert(countIdx + 1, newLine); }
                else { lines.Insert(gen + 1, "ISOPaths = 1"); lines.Insert(gen + 2, newLine); }
                File.Copy(ini, ini + ".bak", true);
                File.WriteAllLines(ini, lines);
                Log("Added the game folder to Dolphin's game list.");
            }
            catch (Exception e) { Log("Couldn't add to Dolphin's game list: " + e.Message); }
        }

        void OnlineHelp()
        {
            MessageBox.Show(this,
                "Playing together (Dolphin NetPlay, works over Hamachi):\n\n" +
                "1. Everyone opens this launcher first so they have the same, up-to-date game.\n" +
                "2. Everyone joins the same Hamachi network.\n" +
                "3. Host: in Dolphin, Tools → Start NetPlay… → Host tab. Set Connection Type to \"Direct Connection\", " +
                "port 2626, pick \"Zoids Combo: Vs. III + Battle Legends\", click Host.\n" +
                "4. Friends: Tools → Start NetPlay… → Connect tab. Connection Type \"Direct Connection\", " +
                "IP = the host's Hamachi IPv4 address (right-click the host in Hamachi → Copy IPv4), port 2626, Connect.\n" +
                "5. Host assigns controller ports, then clicks Start.\n\n" +
                "Everyone needs the same Dolphin version.",
                "Play online");
        }
    }
}
