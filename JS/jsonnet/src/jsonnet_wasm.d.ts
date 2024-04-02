/* tslint:disable */
/* eslint-disable */
/**
* @returns {number}
*/
export function jsonnet_make(): number;
/**
* @param {number} vm
*/
export function jsonnet_destroy(vm: number): void;
/**
* @param {number} vm
* @param {string} filename
* @param {string} snippet
* @returns {string}
*/
export function jsonnet_evaluate_snippet(vm: number, filename: string, snippet: string): string;
/**
* @param {number} vm
* @param {string} filename
* @returns {string}
*/
export function jsonnet_evaluate_file(vm: number, filename: string): string;
/**
* @param {number} vm
* @param {string} key
* @param {string} value
*/
export function ext_string(vm: number, key: string, value: string): void;
/**
* @param {Function} callback
*/
export function execute_callback(callback: Function): void;
/**
*/
export class CallBackClass {
  free(): void;
/**
*/
  constructor();
/**
* @param {Function} f
* @returns {any}
*/
  call_native_js_func(f: Function): any;
/**
*/
  arg: string;
}
