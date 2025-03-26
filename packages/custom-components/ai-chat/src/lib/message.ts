export default interface Message {
    role: 'user' | 'system';
    content: string;
}
