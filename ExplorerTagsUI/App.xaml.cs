using System.Windows;

namespace ExplorerTagsUI
{
    public partial class App : Application
    {
        public static string[] TargetFolders { get; private set; } = new string[0];

        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);
            TargetFolders = e.Args; // Сохраняем пути папок из командной строки
        }
    }
}