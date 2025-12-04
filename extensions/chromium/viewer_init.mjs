import * as pdfjsLib from './pdf.mjs';
import { EventBus, PDFViewer } from './pdf_viewer.mjs';

// Configure worker
pdfjsLib.GlobalWorkerOptions.workerSrc = './pdf.worker.mjs';

// Get the PDF URL from query parameters
function getPdfUrl() {
    const params = new URLSearchParams(window.location.search);
    let file = params.get('file');
    
    // Handle both encoded and non-encoded URLs
    if (file) {
        try {
            // If the URL looks encoded, decode it
            if (file.includes('%')) {
                file = decodeURIComponent(file);
            }
        } catch (e) {
            // If decoding fails, use as-is
        }
    }
    return file;
}

async function initViewer() {
    const loadingMessage = document.getElementById('loadingMessage');
    const errorMessage = document.getElementById('errorMessage');
    const container = document.getElementById('viewerContainer');
    
    const pdfUrl = getPdfUrl();
    
    if (!pdfUrl) {
        loadingMessage.style.display = 'none';
        errorMessage.textContent = 'No PDF file specified. Add ?file=URL to the viewer URL.';
        errorMessage.style.display = 'block';
        return;
    }

    try {
        // Create event bus
        const eventBus = new EventBus();

        // Create PDF viewer
        const pdfViewer = new PDFViewer({
            container: container,
            eventBus: eventBus,
        });

        // Expose for content scripts
        window.PDFViewerApplication = {
            pdfViewer: pdfViewer,
            eventBus: eventBus,
            url: pdfUrl,
        };

        // Load the PDF document
        const loadingTask = pdfjsLib.getDocument({
            url: pdfUrl,
            enableXfa: true,
        });

        const pdfDocument = await loadingTask.promise;
        
        // Set the document
        pdfViewer.setDocument(pdfDocument);
        
        // Update title
        document.title = pdfUrl.split('/').pop() || 'PDF Document';

        // Hide loading message
        loadingMessage.style.display = 'none';

        // Handle resize
        window.addEventListener('resize', () => {
            pdfViewer.currentScaleValue = 'auto';
        });

        // Initial scale
        eventBus.on('pagesinit', () => {
            pdfViewer.currentScaleValue = 'auto';
        });

    } catch (error) {
        console.error('Error loading PDF:', error);
        loadingMessage.style.display = 'none';
        errorMessage.textContent = `Error loading PDF: ${error.message}`;
        errorMessage.style.display = 'block';
    }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initViewer);
} else {
    initViewer();
}