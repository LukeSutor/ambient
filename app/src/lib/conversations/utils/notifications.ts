/**
 * Notification utilities for user feedback
 * Using console for now - can be upgraded to toast library later
 */

/**
 * Shows an error notification to the user
 * @param message - Error message to display
 */
export function showErrorToast(message: string): void {
  console.error('[Toast]', message);
  // TODO: Integrate with toast library (e.g., sonner) when added
  // toast.error(message);
}

/**
 * Shows a success notification to the user
 * @param message - Success message to display
 */
export function showSuccessToast(message: string): void {
  console.log('[Toast]', message);
  // TODO: Integrate with toast library (e.g., sonner) when added
  // toast.success(message);
}

/**
 * Shows an info notification to the user
 * @param message - Info message to display
 */
export function showInfoToast(message: string): void {
  console.info('[Toast]', message);
  // TODO: Integrate with toast library (e.g., sonner) when added
  // toast.info(message);
}
