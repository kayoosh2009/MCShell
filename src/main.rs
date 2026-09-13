use slint_center_win::center_window;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;

    // Окно нужно показать до центрирования — иначе размер ещё не посчитан
    window.show()?;
    center_window(window.window());

    window.run()
}