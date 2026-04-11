//! A2UI Type Definitions

/**
 * Agent-to-User message types
 */
export type A2UIMessageType = 'ask' | 'inform' | 'confirm' | 'select';

/**
 * Inform message levels
 */
export type InformLevel = 'info' | 'warning' | 'error' | 'success';

/**
 * User-to-Agent message types
 */
export type U2AMessageType = 'response' | 'select' | 'confirm' | 'cancel';

/**
 * A2UI Message content
 */
export interface A2UIMessageContent {
  type: A2UIMessageType;
  question?: string;
  context?: string;
  message?: string;
  level?: InformLevel;
  prompt?: string;
  details?: string;
  options?: string[];
}

/**
 * User response
 */
export interface U2AMessage {
  type: U2AMessageType;
  value?: string | number;
}

/**
 * Interactive message envelope
 */
export interface InteractiveMessage {
  id: string;
  session_id: string;
  execution_id: string;
  content: A2UIMessageContent;
  source: string;
  timestamp: string;
  pending: boolean;
  response?: U2AMessage;
  responded_at?: string;
}

/**
 * Messages response
 */
export interface MessagesResponse {
  messages: InteractiveMessage[];
  pending_count: number;
}

/**
 * User response request
 */
export interface UserResponseRequest {
  message_id: string;
  response: U2AMessage;
}

/**
 * A2UI Session
 */
export interface A2UISession {
  id: string;
  execution_id: string;
  state: 'waiting' | 'processing' | 'responded' | 'ended';
}

/**
 * WebSocket event types
 */
export interface A2UIWsEvent {
  type: 'message' | 'pending' | 'response' | 'ended' | 'error';
  data?: InteractiveMessage;
  message_id?: string;
  response?: U2AMessage;
  execution_id?: string;
  error?: {
    code: string;
    message: string;
  };
}

/**
 * A2UI Panel props
 */
export interface A2UIPanelProps {
  executionId: string;
  className?: string;
}