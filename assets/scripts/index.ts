import 'bootstrap';
import Clipboard from 'clipboard';

window.addEventListener('DOMContentLoaded', initializePage);

/// Initializes page content.
function initializePage(e: Event): void {
    new Clipboard('.clipboard');
}
