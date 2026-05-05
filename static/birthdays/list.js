// static/birthdays/list.js

export function init() {
    console.log('Initializing birthdays list page script.');

    // Attach event listener to all delete forms
    document.querySelectorAll('.delete-form').forEach(form => {
        form.addEventListener('submit', (event) => {
            const birthdayName = form.closest('tr').querySelector('td[data-label="Name"]').textContent;
            if (!confirm(`Delete ${birthdayName}?`)) {
                event.preventDefault(); // Prevent form submission if user cancels
            }
        })
    });
}