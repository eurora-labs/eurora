import { InjectionToken } from '@eurora/shared/context';

export const DOWNLOAD_SERVICE = new InjectionToken<DownloadService>('DOWNLOAD_SERVICE');

export class DownloadService {
	constructor() {}
}
