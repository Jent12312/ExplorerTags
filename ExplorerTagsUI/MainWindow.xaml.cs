using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows;
using Microsoft.Data.Sqlite;

namespace ExplorerTagsUI
{
    public class IconItem
    {
        public string Title { get; set; } = "";
        public string ImagePath { get; set; } = "";
    }

    public partial class MainWindow : Window
    {
        private string _dbPath;
        private string _iconsDir;
        private List<string> _tagsList = new List<string>();

        public MainWindow()
        {
            InitializeComponent();

            string appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            _iconsDir = Path.Combine(appData, "ExplorerTags", "Icons");
            _dbPath = Path.Combine(appData, "ExplorerTags", "database.db");

            Directory.CreateDirectory(_iconsDir);

            int count = App.TargetFolders.Length;
            TxtHeader.Text = $"Выделено папок: {count}. Выберите цвет:";

            // 🔥 СЕКРЕТ МГНОВЕННОГО СТАРТА: 
            // Загружаем иконки ПОСЛЕ того, как окно уже отрендерилось на экране!
            Loaded += (s, e) =>
            {
                LoadIcons();
                LoadTags();
            };
        }

        private void LoadIcons()
        {
            var items = new List<IconItem>();
            if (Directory.Exists(_iconsDir))
            {
                foreach (var file in Directory.GetFiles(_iconsDir, "*.ico"))
                {
                    items.Add(new IconItem
                    {
                        Title = Path.GetFileNameWithoutExtension(file),
                        ImagePath = file
                    });
                }
            }
            LstIcons.ItemsSource = items;
        }

        private void LoadTags()
        {
            _tagsList.Clear();
            CmbTags.Items.Clear();
            CmbTags.Items.Add("--- Без тега ---");

            if (File.Exists(_dbPath))
            {
                using (var conn = new SqliteConnection($"Data Source={_dbPath}"))
                {
                    conn.Open();
                    var cmd = conn.CreateCommand();
                    cmd.CommandText = "SELECT name FROM custom_tags ORDER BY id ASC";
                    using (var reader = cmd.ExecuteReader())
                    {
                        while (reader.Read())
                        {
                            string tag = reader.GetString(0);
                            _tagsList.Add(tag);
                            CmbTags.Items.Add(tag);
                        }
                    }
                }
            }
            CmbTags.SelectedIndex = 0;
        }

        private void BtnApply_Click(object sender, RoutedEventArgs e)
        {
            var selectedIcon = LstIcons.SelectedItem as IconItem;
            int tagIdx = CmbTags.SelectedIndex;
            string selectedTag = (tagIdx > 0) ? _tagsList[tagIdx - 1] : null;

            foreach (var folder in App.TargetFolders)
            {
                if (selectedIcon != null)
                {
                    SetFolderColor(folder, selectedIcon.ImagePath);
                }
                if (selectedTag != null)
                {
                    SetFolderTag(folder, selectedTag);
                    SaveTagToDb(folder, selectedTag);
                }
            }
            Close();
        }

        private void BtnReset_Click(object sender, RoutedEventArgs e)
        {
            foreach (var folder in App.TargetFolders)
            {
                ResetFolder(folder);
                SaveTagToDb(folder, "");
            }
            Close();
        }

        private void BtnNewTag_Click(object sender, RoutedEventArgs e)
        {
            string newTag = PromptDialog.Show("Введите название нового тега:", "Новый тег");
            if (!string.IsNullOrWhiteSpace(newTag))
            {
                using (var conn = new SqliteConnection($"Data Source={_dbPath}"))
                {
                    conn.Open();
                    var cmd = conn.CreateCommand();
                    cmd.CommandText = "INSERT OR IGNORE INTO custom_tags (name) VALUES (@name)";
                    cmd.Parameters.AddWithValue("@name", newTag);
                    cmd.ExecuteNonQuery();
                }
                LoadTags();
                CmbTags.SelectedItem = newTag;
            }
        }

        // 🔥 НОВАЯ ФУНКЦИЯ: УДАЛЕНИЕ ТЕГА
        private void BtnDeleteTag_Click(object sender, RoutedEventArgs e)
        {
            int tagIdx = CmbTags.SelectedIndex;
            if (tagIdx <= 0)
            {
                MessageBox.Show("Выберите тег для удаления из списка!", "Удаление", MessageBoxButton.OK, MessageBoxImage.Information);
                return;
            }

            string tagToDelete = _tagsList[tagIdx - 1];

            var result = MessageBox.Show($"Вы уверены, что хотите полностью удалить тег \"{tagToDelete}\"?", "Подтверждение", MessageBoxButton.YesNo, MessageBoxImage.Question);
            if (result == MessageBoxResult.Yes)
            {
                using (var conn = new SqliteConnection($"Data Source={_dbPath}"))
                {
                    conn.Open();
                    var cmd = conn.CreateCommand();
                    cmd.CommandText = "DELETE FROM custom_tags WHERE name = @name";
                    cmd.Parameters.AddWithValue("@name", tagToDelete);
                    cmd.ExecuteNonQuery();
                }
                LoadTags(); // Перезагружаем список
            }
        }

        private void BtnCancel_Click(object sender, RoutedEventArgs e) => Close();

        private void SetFolderColor(string folderPath, string iconPath)
        {
            var fcs = new NativeMethods.SHFOLDERCUSTOMSETTINGS();
            fcs.dwSize = (uint)Marshal.SizeOf(fcs);
            fcs.dwMask = NativeMethods.FCSM_ICONFILE;
            fcs.pszIconFile = iconPath;
            fcs.iIconIndex = 0;

            NativeMethods.SHGetSetFolderCustomSettings(ref fcs, folderPath, NativeMethods.FCS_FORCEWRITE);
            NotifyShell(folderPath);
        }

        private void SetFolderTag(string folderPath, string tagText)
        {
            string iniPath = Path.Combine(folderPath, "desktop.ini");
            EnsureUnicodeIni(iniPath);

            NativeMethods.WritePrivateProfileString(".ShellClassInfo", "InfoTip", tagText, iniPath);
            NativeMethods.WritePrivateProfileString("{F29F85E0-4FF9-1068-AB91-08002B27B3D9}", "Prop5", "31," + tagText, iniPath);
            NativeMethods.WritePrivateProfileString("ExplorerTags", "ProjectTag", tagText, iniPath);
            NativeMethods.WritePrivateProfileString(null, null, null, iniPath);

            File.SetAttributes(iniPath, FileAttributes.Hidden | FileAttributes.System);
            new DirectoryInfo(folderPath).Attributes |= FileAttributes.ReadOnly;

            NotifyShell(folderPath);
        }

        private void ResetFolder(string folderPath)
        {
            var fcs = new NativeMethods.SHFOLDERCUSTOMSETTINGS();
            fcs.dwSize = (uint)Marshal.SizeOf(fcs);
            fcs.dwMask = NativeMethods.FCSM_ICONFILE | NativeMethods.FCSM_INFOTIP;
            fcs.pszIconFile = "";
            fcs.pszInfoTip = "";

            NativeMethods.SHGetSetFolderCustomSettings(ref fcs, folderPath, NativeMethods.FCS_FORCEWRITE);

            string iniPath = Path.Combine(folderPath, "desktop.ini");
            NativeMethods.WritePrivateProfileString("{F29F85E0-4FF9-1068-AB91-08002B27B3D9}", null, null, iniPath);
            NativeMethods.WritePrivateProfileString("ExplorerTags", null, null, iniPath);
            NativeMethods.WritePrivateProfileString(null, null, null, iniPath);

            new DirectoryInfo(folderPath).Attributes &= ~FileAttributes.ReadOnly;
            NotifyShell(folderPath);
        }

        private void SaveTagToDb(string folderPath, string tag)
        {
            using (var conn = new SqliteConnection($"Data Source={_dbPath}"))
            {
                conn.Open();
                var cmd = conn.CreateCommand();
                cmd.CommandText = "INSERT INTO folder_tags (path, tag) VALUES (@path, @tag) ON CONFLICT(path) DO UPDATE SET tag = excluded.tag;";
                cmd.Parameters.AddWithValue("@path", folderPath.ToLower());
                cmd.Parameters.AddWithValue("@tag", tag);
                cmd.ExecuteNonQuery();
            }
        }

        private void EnsureUnicodeIni(string iniPath)
        {
            if (!File.Exists(iniPath))
            {
                File.WriteAllBytes(iniPath, new byte[] { 0xFF, 0xFE });
            }
        }

        private void NotifyShell(string folderPath)
        {
            NativeMethods.SHChangeNotify(NativeMethods.SHCNE_UPDATEITEM, NativeMethods.SHCNF_PATHW, folderPath, null);
            NativeMethods.SHChangeNotify(NativeMethods.SHCNE_UPDATEDIR, NativeMethods.SHCNF_PATHW, Path.GetDirectoryName(folderPath), null);
        }
    }

    public static class PromptDialog
    {
        public static string Show(string text, string caption)
        {
            Window prompt = new Window()
            {
                Width = 360,
                Height = 170,
                Title = caption,
                WindowStartupLocation = WindowStartupLocation.CenterScreen,
                ResizeMode = ResizeMode.NoResize,
                Background = System.Windows.Media.Brushes.DarkGray
            };

            System.Windows.Controls.Label lbl = new System.Windows.Controls.Label() { Content = text };
            System.Windows.Controls.Canvas.SetLeft(lbl, 15);
            System.Windows.Controls.Canvas.SetTop(lbl, 10);

            System.Windows.Controls.TextBox txt = new System.Windows.Controls.TextBox() { Width = 310, Height = 25 };
            System.Windows.Controls.Canvas.SetLeft(txt, 20);
            System.Windows.Controls.Canvas.SetTop(txt, 40);

            System.Windows.Controls.Button btn = new System.Windows.Controls.Button() { Content = "ОК", Width = 80, Height = 26, IsDefault = true };
            System.Windows.Controls.Canvas.SetLeft(btn, 250);
            System.Windows.Controls.Canvas.SetTop(btn, 80);

            btn.Click += (sender, e) => { prompt.DialogResult = true; prompt.Close(); };

            System.Windows.Controls.Canvas canvas = new System.Windows.Controls.Canvas();
            canvas.Children.Add(lbl); 
            canvas.Children.Add(txt); 
            canvas.Children.Add(btn);
            prompt.Content = canvas;

            return prompt.ShowDialog() == true ? txt.Text : "";
        }
    }

    internal static class NativeMethods
    {
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        public struct SHFOLDERCUSTOMSETTINGS
        {
            public uint dwSize;
            public uint dwMask;
            public IntPtr pvid;
            public string pszWebViewTemplate;
            public uint cchWebViewTemplate;
            public string pszWebViewTemplateVersion;
            public string pszInfoTip;
            public uint cchInfoTip;
            public IntPtr pclsid;
            public uint dwFlags;
            public string pszIconFile;
            public uint cchIconFile;
            public int iIconIndex;
            public string pszLogo;
            public uint cchLogo;
        }

        public const uint FCSM_ICONFILE = 0x00000010;
        public const uint FCSM_INFOTIP = 0x00000004;
        public const uint FCS_FORCEWRITE = 0x00000002;
        public const uint SHCNE_UPDATEITEM = 0x00002000;
        public const uint SHCNE_UPDATEDIR = 0x00001000;
        public const uint SHCNF_PATHW = 0x0005;

        [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
        public static extern int SHGetSetFolderCustomSettings(ref SHFOLDERCUSTOMSETTINGS pfcs, string pszPath, uint dwReadWrite);

        [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
        public static extern void SHChangeNotify(uint wEventId, uint uFlags, string dwItem1, string dwItem2);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        public static extern bool WritePrivateProfileString(string section, string key, string val, string filePath);
    }
}