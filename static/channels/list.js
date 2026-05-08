// static/channels/list.js

export function init() {
    document.querySelectorAll('.channel-remove-form').forEach((form) => {
        form.addEventListener('submit', (e) => {
            const message = form.dataset.confirm || 'Remove this channel?';
            if (!window.confirm(message)) {
                e.preventDefault();
            }
        });
    });
}