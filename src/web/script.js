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

    // 1. Применение фонового изображения с затемнением
    function applyBackground(url) {
        document.body.style.backgroundImage = `linear-gradient(rgba(18, 18, 20, 0.82), rgba(18, 18, 20, 0.82)), url("${url}")`;
    }

    // 2. Модальное окно выбора обоев
    const bgModal = document.getElementById('bg-modal');
    const openBgModalBtn = document.getElementById('open-bg-modal-btn');
    const closeBgModalBtn = document.getElementById('close-bg-modal-btn');
    const bgInput = document.getElementById('bg-file-input');
    const customBgThumb = document.getElementById('custom-bg-thumb');

    if (openBgModalBtn && bgModal) {
        openBgModalBtn.addEventListener('click', () => bgModal.classList.remove('hidden'));
        closeBgModalBtn.addEventListener('click', () => bgModal.classList.add('hidden'));
        bgModal.addEventListener('click', (e) => {
            if (e.target === bgModal) bgModal.classList.add('hidden');
        });
    }

    // Выбор обоев из карточек
    document.querySelectorAll('.wallpaper-item[data-bg]').forEach(item => {
        item.addEventListener('click', () => {
            const bgUrl = item.dataset.bg;
            applyBackground(bgUrl);
            localStorage.setItem('mcshell_bg', bgUrl);
            bgModal.classList.add('hidden');
        });
    });

    // Загрузка нового файла обоев
    if (bgInput) {
        bgInput.addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (!file) return;

            const localUrl = URL.createObjectURL(file);
            applyBackground(localUrl);
            localStorage.setItem('mcshell_bg', `/api/background?t=${token}`);
            if (customBgThumb) customBgThumb.src = localUrl;
            bgModal.classList.add('hidden');

            try {
                await fetch(`/api/background?t=${token}`, {
                    method: 'POST',
                    body: file
                });
            } catch (err) {
                console.error('Failed to save background:', err);
            }
        });
    }

    // Восстановление выбранного фона при старте (по умолчанию elements/def-backgroud.png)
    function loadBackground() {
        const saved = localStorage.getItem('mcshell_bg') || '/elements/def-backgroud.png';
        applyBackground(saved);
    }

    loadProfile();
    loadArt();
    loadBackground();

    // 3. Выбор скина, превью в 3D и сохранение в папку игры
    const skinInput = document.getElementById('skin-file-input');
    skinInput.addEventListener('change', async (e) => {
        const file = e.target.files[0];
        if (!file) return;

        // Мгновенный показ в 3D
        const reader = new FileReader();
        reader.onload = (event) => {
            if (skinViewer) {
                skinViewer.loadSkin(event.target.result);
            }
        };
        reader.readAsDataURL(file);

        // Отправка файла в бэкенд для сохранения в папку игры
        try {
            await fetch(`/api/skin?t=${token}`, {
                method: 'POST',
                body: file
            });
            console.log('Skin saved to Minecraft game folder!');
        } catch (err) {
            console.error('Failed to upload skin to game directory:', err);
        }
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

    // === 4. Управление версиями (Избранное, Удаление, Фоновая загрузка) ===

    // Работа с избранными версиями (сохраняем в localStorage)
    function getFavorites() {
        try {
            return JSON.parse(localStorage.getItem('mcshell_fav_versions') || '[]');
        } catch (_) {
            return [];
        }
    }

    function toggleFavorite(id) {
        let favs = getFavorites();
        if (favs.includes(id)) {
            favs = favs.filter(v => v !== id);
        } else {
            favs.push(id);
        }
        localStorage.setItem('mcshell_fav_versions', JSON.stringify(favs));
        loadVersions();
    }

    // Модальное окно подтверждения удаления
    const confirmModal = document.getElementById('confirm-modal');
    const confirmDeleteText = document.getElementById('confirm-delete-text');
    const confirmDeleteBtn = document.getElementById('confirm-delete-btn');
    const cancelDeleteBtn = document.getElementById('cancel-delete-btn');
    let versionPendingDelete = null;

    if (cancelDeleteBtn && confirmModal) {
        cancelDeleteBtn.addEventListener('click', () => {
            confirmModal.classList.add('hidden');
            versionPendingDelete = null;
        });
        confirmModal.addEventListener('click', (e) => {
            if (e.target === confirmModal) {
                confirmModal.classList.add('hidden');
                versionPendingDelete = null;
            }
        });
    }

    if (confirmDeleteBtn) {
        confirmDeleteBtn.addEventListener('click', async () => {
            if (!versionPendingDelete) return;
            const idToDelete = versionPendingDelete;
            confirmDeleteBtn.textContent = 'Deleting...';
            try {
                await api(`/api/versions/delete?id=${encodeURIComponent(idToDelete)}`, {
                    method: 'POST'
                });
            } catch (err) {
                console.error('Failed to delete version:', err);
            }
            confirmModal.classList.add('hidden');
            confirmDeleteBtn.textContent = 'Delete';
            versionPendingDelete = null;
            loadVersions();
        });
    }

    function promptDeleteVersion(id) {
        versionPendingDelete = id;
        if (confirmDeleteText) {
            confirmDeleteText.textContent = `Are you sure you want to delete version "${id}"?`;
        }
        if (confirmModal) {
            confirmModal.classList.remove('hidden');
        }
    }

    // Загрузка и рендер установленных версий (с подъёмом избранных наверх)
    async function loadVersions() {
        const listEl = document.getElementById('installed-versions-list');
        if (!listEl) return;

        try {
            const targets = await api('/api/targets');
            if (!targets || targets.length === 0) {
                listEl.innerHTML = '<span class="empty-text">No versions installed yet. Install one above!</span>';
                return;
            }

            const favorites = getFavorites();

            // Сортируем: избранные версии всегда идут первыми
            targets.sort((a, b) => {
                const aFav = favorites.includes(a.id);
                const bFav = favorites.includes(b.id);
                if (aFav && !bFav) return -1;
                if (!aFav && bFav) return 1;
                return a.id.localeCompare(b.id);
            });

            listEl.innerHTML = '';
            targets.forEach(item => {
                const isFabric = item.loader === 'fabric';
                const isFav = favorites.includes(item.id);

                const card = document.createElement('div');
                card.className = 'version-card';
                card.innerHTML = `
                    <div class="version-info">
                        <span class="version-name">${item.id}</span>
                        <span class="version-badge ${isFabric ? 'fabric' : ''}">${isFabric ? 'Fabric' : 'Vanilla'}</span>
                    </div>
                    <div class="version-actions">
                        <button class="icon-action-btn star-btn ${isFav ? 'active' : ''}" title="${isFav ? 'Remove from favorites' : 'Add to favorites'}">
                            ${isFav ? '★' : '☆'}
                        </button>
                        <button class="icon-action-btn delete-btn" title="Delete version">
                            <img src="/elements/delete.png" alt="Delete">
                        </button>
                    </div>
                `;

                // Обработчик звезды
                const starBtn = card.querySelector('.star-btn');
                starBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    toggleFavorite(item.id);
                });

                // Обработчик удаления
                const deleteBtn = card.querySelector('.delete-btn');
                deleteBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    promptDeleteVersion(item.id);
                });

                listEl.appendChild(card);
            });
        } catch (err) {
            console.error('Failed to load versions:', err);
            listEl.innerHTML = '<span class="empty-text">Error loading versions</span>';
        }
    }

    // === Виджет фоновой загрузки (в стиле IDE) с реальными байтами и скоростью ===
    const downloadWidget = document.getElementById('download-widget');
    const downloadTitle = document.getElementById('download-title');
    const downloadPercent = document.getElementById('download-percent');
    const downloadBarFill = document.getElementById('download-bar-fill');
    const downloadBytes = document.getElementById('download-bytes');
    const downloadSpeed = document.getElementById('download-speed');

    function formatBytes(bytes) {
        if (!bytes || bytes <= 0) return '0 B';
        const units = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i] || 'MB'}`;
    }

    let isPollingProgress = false;

    async function pollProgress() {
        if (isPollingProgress) return;
        isPollingProgress = true;

        const check = async () => {
            try {
                const data = await api('/api/progress');
                if (data && data.active) {
                    if (downloadWidget) downloadWidget.classList.remove('hidden');

                    const downloaded = data.downloaded || 0;
                    const total = data.total || 0;
                    const speed = data.speed || 0;
                    const task = data.task || 'Minecraft';

                    const percent = total > 0 ? Math.min(100, (downloaded / total) * 100) : 0;

                    if (downloadTitle) downloadTitle.textContent = `Installing ${task}`;
                    if (downloadPercent) downloadPercent.textContent = `${Math.round(percent)}%`;
                    if (downloadBarFill) downloadBarFill.style.width = `${percent}%`;

                    if (downloadBytes) {
                        downloadBytes.textContent = total > 0 
                            ? `${formatBytes(downloaded)} / ${formatBytes(total)}` 
                            : formatBytes(downloaded);
                    }
                    if (downloadSpeed) {
                        downloadSpeed.textContent = speed > 0 ? `${formatBytes(speed)}/s` : '0 B/s';
                    }

                    setTimeout(check, 300);
                } else if (downloadWidget && !downloadWidget.classList.contains('hidden')) {
                    // Загрузка завершена
                    if (downloadTitle) downloadTitle.textContent = 'Installed!';
                    if (downloadPercent) downloadPercent.textContent = '100%';
                    if (downloadBarFill) downloadBarFill.style.width = '100%';
                    if (downloadSpeed) downloadSpeed.textContent = 'Done';

                    setTimeout(() => {
                        downloadWidget.classList.add('hidden');
                        isPollingProgress = false;
                        loadVersions();
                    }, 1800);
                } else {
                    isPollingProgress = false;
                }
            } catch (err) {
                console.error('Error polling progress:', err);
                isPollingProgress = false;
            }
        };

        check();
    }

    // === Модалка установки версий с официального манифеста Mojang ===
    const installModal = document.getElementById('install-modal');
    const openInstallModalBtn = document.getElementById('open-install-modal-btn');
    const closeInstallModalBtn = document.getElementById('close-install-modal-btn');
    const availableList = document.getElementById('available-versions-list');

    if (openInstallModalBtn && installModal) {
        openInstallModalBtn.addEventListener('click', () => {
            installModal.classList.remove('hidden');
            loadAvailableVersions();
        });
    }

    if (closeInstallModalBtn && installModal) {
        closeInstallModalBtn.addEventListener('click', () => installModal.classList.add('hidden'));
        installModal.addEventListener('click', (e) => {
            if (e.target === installModal) installModal.classList.add('hidden');
        });
    }

    async function loadAvailableVersions() {
        if (!availableList) return;
        availableList.innerHTML = '<span class="empty-text">Loading release versions...</span>';

        try {
            // Подтягиваем официальный манифест версий
            const res = await fetch('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json');
            const data = await res.json();
            
            // Фильтруем только стабильные релизы
            const releases = (data.versions || []).filter(v => v.type === 'release');

            availableList.innerHTML = '';
            releases.slice(0, 40).forEach(ver => {
                const row = document.createElement('div');
                row.className = 'install-version-row';
                row.innerHTML = `
                    <div class="version-info">
                        <span class="version-name">${ver.id}</span>
                        <span class="version-badge">Release</span>
                    </div>
                    <button class="btn btn-primary start-install-btn" data-ver="${ver.id}">Install</button>
                `;

                row.querySelector('.start-install-btn').addEventListener('click', () => {
                    installModal.classList.add('hidden');
                    startVersionDownload(ver.id);
                });

                availableList.appendChild(row);
            });
        } catch (err) {
            console.error('Failed to load version manifest:', err);
            availableList.innerHTML = '<span class="empty-text">Failed to fetch versions list.</span>';
        }
    }

    // Запуск процесса загрузки с плашкой в боковом меню
    async function startVersionDownload(versionId) {
        updateDownloadProgress(`Installing ${versionId}`, 5);

        try {
            await api(`/api/install?version=${encodeURIComponent(versionId)}`, { method: 'POST' });

            let current = 10;
            const interval = setInterval(() => {
                current += Math.random() * 18;
                if (current >= 100) {
                    current = 100;
                    clearInterval(interval);
                }
                updateDownloadProgress(`Installing ${versionId}`, current);
            }, 300);
        } catch (err) {
            console.error('Install failed:', err);
            updateDownloadProgress('Install Error', 100);
        }
    }

    // 5. Навигация в боковом меню
    const navButtons = document.querySelectorAll('.nav-btn');
    navButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            navButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            const page = btn.dataset.page;
            
            // Скрываем все вкладки
            document.querySelectorAll('.tab-content').forEach(tab => tab.classList.add('hidden'));

            // Показываем выбранную
            const activeTab = document.getElementById(`tab-${page}`);
            if (activeTab) {
                activeTab.classList.remove('hidden');
            }

            if (page === 'versions') {
                loadVersions();
            }
        });
    });
});