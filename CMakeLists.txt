cmake_minimum_required(VERSION 3.15)
project(ExplorerTags VERSION 1.0 LANGUAGES C CXX)

# Используем современный стандарт C++17 (или 20)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Собираем DLL (SHARED)
add_library(${PROJECT_NAME} SHARED
    src/DllMain.cpp
    src/Global.cpp
    src/ClassFactory.cpp
    src/MainMenuCommand.cpp
    src/Colorizer.cpp   
    src/CommandEnum.cpp     
    src/Exports.def
    src/PropertyHandler.cpp
    src/PropertyHandler.cpp 
    src/Database.cpp        
    src/sqlite3.c         
)

# Указываем папки с заголовочными файлами
target_include_directories(${PROJECT_NAME} PRIVATE include)

# Подключаем необходимые системные библиотеки Windows
target_link_libraries(${PROJECT_NAME} PRIVATE
    shell32   # Для работы с проводником
    ole32     # Для базового COM (CoCreateInstance и т.д.)
    oleaut32  # Для работы с COM-типами данных
    uuid      # Для GUID-ов
    shlwapi   # Для удобной работы с путями (PathFileExists и т.д.)
    propsys   # Для Property Handler (группировки)
)

# Настройки для MSVC (многопоточная DLL, строгие предупреждения)
if(MSVC)
    target_compile_options(${PROJECT_NAME} PRIVATE /W4 /utf-8)
endif()