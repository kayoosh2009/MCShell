// Извлекаем токен из параметров URL
const urlParams = new URLSearchParams(window.location.search);
const token = urlParams.get('t') || '';

// Хелпер для запросов к нашему Rust бэкенду
async function api(path, options = {}) {
    const sep = path.includes('?') ? '&' : '?';
    const url = `${path}${sep}t=${token}`;
    const res = await fetch(url, options);
    return res.json();
}

document.addEventListener('DOMContentLoaded', () => {
    // 1. Навигация по вкладкам справа
    const navButtons = document.querySelectorAll('.nav-btn');
    navButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            navButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            const page = btn.dataset.page;
            console.log('Переключение на страницу:', page);
            // Сюда будем добавлять переключение секций по мере их создания
        });
    });

    // 2. Локальное превью выбранного скина
    const skinInput = document.getElementById('skin-file-input');
    const previewContainer = document.getElementById('skin-preview-container');

    skinInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = (event) => {
            previewContainer.innerHTML = '';
            
            const img = document.createElement('img');
            img.src = event.target.result;
            img.alt = 'Skin preview';
            img.style.maxWidth = '100%';
            img.style.maxHeight = '100%';
            img.style.imageRendering = 'pixelated'; // Чтобы пиксели не размывались
            img.style.borderRadius = '4px';

            previewContainer.appendChild(img);
        };
        reader.readAsDataURL(file);
    });

    // 3. Сохранение имени игрока
    const saveUsernameBtn = document.getElementById('save-username-btn');
    const usernameInput = document.getElementById('username');

    saveUsernameBtn.addEventListener('click', async () => {
        const username = usernameInput.value.trim();
        if (!username) return;

        saveUsernameBtn.textContent = 'Сохранение...';
        try {
            await api('/api/profile', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username })
            });
            saveUsernameBtn.textContent = 'Сохранено!';
            setTimeout(() => { saveUsernameBtn.textContent = 'Сохранить'; }, 1500);
        } catch (err) {
            console.error('Ошибка сохр