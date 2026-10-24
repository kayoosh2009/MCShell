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
    const usernameInput = document.getElementById('username');
    const skinCanvas = document.getElementById('skin-viewer-canvas');

    // 1. Инициализация 3D Skin Viewer
    let skinViewer = null;
    if (window.skinview3d && skinCanvas) {
        skinViewer = new skinview3d.SkinViewer({
            canvas: skinCanvas,
            width: 240,
            height: 340,
        });

        // Плавная анимация стойки персонажа
        skinViewer.animation = new skinview3d.IdleAnimation();
        skinViewer.animation.speed = 0.6;
        
        // Включаем вращение мышью (в v3 встроен напрямую)
        if (skinViewer.controls) {
            skinViewer.controls.enableRotate = true;
            skinViewer.controls.enableZoom = false;
            skinViewer.controls.enablePan = false;
        }
    }

    // 2. Загрузка профиля и статистики с бэкенда
    async function loadProfile() {
        try {
            const data = await api('/api/profile');
            if (data.username) {
                usernameInput.value = data.username;
            }

            // Загружаем скин в 3D модель
            if (skinViewer && data.has_skin) {
                skinViewer.loadSkin(`/api/skin?t=${token}`);
            }

            // Заполняем статистику
            if (data.stats) {
                document.getElementById('stat-playtime').textContent = data.stats.playtime || '-';
                document.getElementById('stat-launches').textContent = data.stats.launches ?? '-';
                document.getElementById('stat-last-played').textContent = data.stats.last_played || '-';
                document.getElementById('stat-worlds').textContent = data.stats.worlds || '-';
                document.getElementById('stat-mods').textContent = data.stats.mods || '-';
                document.getElementById('stat-packs').textContent = data.stats.packs || '-';
            }
        } catch (err) {
            console.error('Failed to load profile:', err);
        }
    }

    async function loadArt() {
        try {
            const data = await api('/api/art');
            const artEl = document.getElementById('ascii-art');
            if (artEl && data.art) {
                artEl.textContent = data.art;
            }
        } catch (err) {
            console.error('Failed to load ascii art:', err);
        }
    }

    loadProfile();
    loadArt();

    // 3. Выбор и локальный предпросмотр скина в 3D
    const skinInput = document.getElementById('skin-file-input');
    skinInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = (event) => {
            if (skinViewer) {
                skinViewer.loadSkin(event.target.result);
            }
        };
        reader.readAsDataURL(file);
    });

    // 4. Сохранение никнейма
    const saveUsernameBtn = document.getElementById('save-username-btn');
    saveUsernameBtn.addEventListener('click', async () => {
        const username = usernameInput.value.trim();
        if (!username) return;

        saveUsernameBtn.textContent = 'Saving...';
        try {
            await api('/api/profile', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username })
            });
            saveUsernameBtn.textContent = 'Saved!';
            setTimeout(() => { saveUsernameBtn.textContent = 'Save'; }, 1500);
        } catch (err) {
            console.error('Failed to save profile:', err);
            saveUsernameBtn.textContent = 'Error';
        }
    });

    // 5. Навигация в боковом меню
    const navButtons = document.querySelectorAll('.nav-btn');
    navButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            navButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            const page = btn.dataset.page;
            console.log('Navigated to:', page);
        });
    });
});