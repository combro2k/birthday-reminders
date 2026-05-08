// static/users/profile.js

export function init() {
    const hiddenInput = document.getElementById('days_before_value');
    const checkboxes = Array.from(document.querySelectorAll('.js-reminder-day'));

    if (!hiddenInput || checkboxes.length === 0) {
        return;
    }

    const syncReminderDays = () => {
        const values = checkboxes
            .filter((checkbox) => checkbox.checked)
            .map((checkbox) => checkbox.value);
        hiddenInput.value = values.join(',');
    };

    for (const checkbox of checkboxes) {
        checkbox.addEventListener('change', syncReminderDays);
    }

    syncReminderDays();
}