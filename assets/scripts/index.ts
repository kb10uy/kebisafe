import 'bootstrap';
import axios from 'axios';

window.addEventListener('DOMContentLoaded', initializePage);

/// Initializes page content.
function initializePage(e: Event): void {
    const forms = document.querySelectorAll('form[data-manual]') as NodeListOf<HTMLFormElement>;

    for (const form of forms) {
        form.addEventListener('submit', (e) => {
            e.preventDefault();

            console.log(`Performing manual redirection of #${form.id}...`);
            processFormRedirect(form);
        });

        console.log(`Form #${form.id} has been set to manual redirection.`);
    }
}

/// Processes form submission resulting into redirection with fetch().
async function processFormRedirect(form: HTMLFormElement): Promise<void> {
    let method = 'POST';
    let token = '';
    const formData = new FormData();

    for (const formElement of form.elements) {
        if (!(formElement instanceof HTMLInputElement)) continue;
        switch (formElement.name) {
            case '_method':
                method = formElement.value;
                break;
            case '_token':
                token = formElement.value;
                formData.append('_token', token);
                break;
            default:
                formData.append(formElement.name, formElement.value);
                break;
        }
    }

    if (token === '') {
        console.error('CSRF token was missing, skipping request.');
        return;
    }

    try {
        const response = await axios({
            method: method as 'POST' | 'PUT' | 'PATCH' | 'DELETE',
            url: form.action,
            data: formData,
            maxRedirects: 0,
            responseType: 'text',
        });
        if (response.status >= 300) {
            location.href = response.headers['location'];
        }
    } catch (e) {
        console.error(`Request error: ${e}`);
    }
}
