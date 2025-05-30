// Export all proto services and types
// Using namespace exports to avoid naming conflicts

export * as AuthService from './lib/auth_service.js';
export * as NativeMessaging from './lib/native_messaging.js';
export * as QuestionsService from './lib/questions_service.js';
export * as TauriIpc from './lib/tauri_ipc.js';

// Export shared types directly since they're commonly used
export * from './lib/shared.js';

// Export OCR service if it exists
export * as OcrService from './lib/gen/ocr_service_pb.js';
export * as OcrServiceClient from './lib/gen/Ocr_serviceServiceClientPb.js';

// Re-export commonly used types for convenience
export type { ProtoImage, ProtoImageFormat } from './lib/shared.js';
