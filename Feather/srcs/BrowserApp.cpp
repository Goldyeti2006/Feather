#include "BrowserApp.h"
#include <Ultralight/Ultralight.h>
#include <AppCore/App.h>
#include <AppCore/Window.h>
#include <AppCore/View.h>
#include <Ultralight/RefCounted.h>
#include <Ultralight/RefPtr.h>

void BrowserApp::Run() {
    Config config;
    Ref<App> app = App::Create(config);

    Ref<Window> window = Window::Create(app->main_monitor(), 1024, 768, false, kWindowFlags_Titled);
    Ref<View> view = app->CreateView(1024, 768, false);

    view->LoadURL("https://google.com");
    window->set_view(view);
    app->Run();
}