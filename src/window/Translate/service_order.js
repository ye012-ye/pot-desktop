export function isFirstEnabledService(serviceInstanceKey, serviceList, configMap) {
    return serviceList.find((key) => configMap[key]?.enable ?? true) === serviceInstanceKey;
}
