import { fetch } from '@tauri-apps/api/http';
import { Button, Input, Switch, Textarea } from '@nextui-org/react';
import { MdDeleteOutline } from 'react-icons/md';
import toast, { Toaster } from 'react-hot-toast';
import { useTranslation } from 'react-i18next';
import React, { useEffect, useRef, useState } from 'react';

import { useConfig } from '../../../hooks/useConfig';
import { useToastStyle } from '../../../hooks';
import { INSTANCE_NAME_CONFIG_KEY } from '../../../utils/service_instance';
import { defaultRequestArguments } from '../openai/Config';
import { createLmStudioHeaders, getModelsUrl, LM_STUDIO_DEFAULT_PROMPT_LIST, parseModelIds } from './api';
import { Language, translate } from './index';

export function Config(props) {
    const { instanceKey, updateServiceList, onClose } = props;
    const { t } = useTranslation();
    const [serviceConfig, setServiceConfig] = useConfig(
        instanceKey,
        {
            [INSTANCE_NAME_CONFIG_KEY]: t('services.translate.lmstudio.title'),
            baseUrl: 'http://localhost:1234/v1',
            model: '',
            apiKey: '',
            stream: false,
            promptList: LM_STUDIO_DEFAULT_PROMPT_LIST,
            requestArguments: defaultRequestArguments,
        },
        { sync: false }
    );
    const [isLoading, setIsLoading] = useState(false);
    const [isLoadingModels, setIsLoadingModels] = useState(false);
    const [models, setModels] = useState([]);
    const [modelError, setModelError] = useState('');
    const didLoadModels = useRef(false);
    const toastStyle = useToastStyle();

    async function loadModels(config = serviceConfig) {
        if (!config) return;
        setIsLoadingModels(true);
        setModelError('');
        try {
            const response = await fetch(getModelsUrl(config.baseUrl), {
                method: 'GET',
                headers: createLmStudioHeaders(config.apiKey),
            });
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            setModels(parseModelIds(response.data));
        } catch {
            setModels([]);
            setModelError(t('services.translate.lmstudio.model_error'));
        } finally {
            setIsLoadingModels(false);
        }
    }

    useEffect(() => {
        if (serviceConfig && !didLoadModels.current) {
            didLoadModels.current = true;
            loadModels(serviceConfig);
        }
    }, [serviceConfig]);

    async function handleSubmit(e) {
        e.preventDefault();
        if (!serviceConfig.model.trim()) {
            toast.error(t('services.translate.lmstudio.model_required'), { style: toastStyle });
            return;
        }

        setIsLoading(true);
        try {
            await translate('hello', Language.auto, Language.zh_cn, { config: serviceConfig });
            setServiceConfig(serviceConfig, true);
            updateServiceList(instanceKey);
            onClose();
        } catch (error) {
            toast.error(t('config.service.test_failed') + error.toString(), { style: toastStyle });
        } finally {
            setIsLoading(false);
        }
    }

    return (
        serviceConfig !== null && (
            <form onSubmit={handleSubmit}>
                <Toaster />
                <div className='config-item'>
                    <Input
                        label={t('services.instance_name')}
                        labelPlacement='outside-left'
                        value={serviceConfig[INSTANCE_NAME_CONFIG_KEY]}
                        variant='bordered'
                        classNames={{
                            base: 'justify-between',
                            label: 'text-[length:--nextui-font-size-medium]',
                            mainWrapper: 'max-w-[50%]',
                        }}
                        onValueChange={(value) => {
                            setServiceConfig({
                                ...serviceConfig,
                                [INSTANCE_NAME_CONFIG_KEY]: value,
                            });
                        }}
                    />
                </div>
                <div className='config-item'>
                    <Switch
                        isSelected={serviceConfig.stream}
                        onValueChange={(value) => {
                            setServiceConfig({ ...serviceConfig, stream: value });
                        }}
                        classNames={{
                            base: 'flex flex-row-reverse justify-between w-full max-w-full',
                        }}
                    >
                        {t('services.translate.lmstudio.stream')}
                    </Switch>
                </div>
                <div className='config-item'>
                    <Input
                        label={t('services.translate.lmstudio.base_url')}
                        labelPlacement='outside-left'
                        value={serviceConfig.baseUrl}
                        variant='bordered'
                        classNames={{
                            base: 'justify-between',
                            label: 'text-[length:--nextui-font-size-medium]',
                            mainWrapper: 'max-w-[50%]',
                        }}
                        onValueChange={(value) => {
                            setServiceConfig({ ...serviceConfig, baseUrl: value });
                        }}
                    />
                </div>
                <div className='config-item'>
                    <Input
                        label={t('services.translate.lmstudio.api_key')}
                        labelPlacement='outside-left'
                        type='password'
                        value={serviceConfig.apiKey}
                        variant='bordered'
                        classNames={{
                            base: 'justify-between',
                            label: 'text-[length:--nextui-font-size-medium]',
                            mainWrapper: 'max-w-[50%]',
                        }}
                        onValueChange={(value) => {
                            setServiceConfig({ ...serviceConfig, apiKey: value });
                        }}
                    />
                </div>
                <div className='config-item'>
                    <Input
                        label={t('services.translate.lmstudio.model')}
                        labelPlacement='outside-left'
                        list='lmstudio-models'
                        value={serviceConfig.model}
                        variant='bordered'
                        classNames={{
                            base: 'justify-between',
                            label: 'text-[length:--nextui-font-size-medium]',
                            mainWrapper: 'max-w-[50%]',
                        }}
                        onValueChange={(value) => {
                            setServiceConfig({ ...serviceConfig, model: value });
                        }}
                        endContent={
                            <Button
                                size='sm'
                                variant='light'
                                isLoading={isLoadingModels}
                                onPress={() => loadModels()}
                            >
                                {t('services.translate.lmstudio.refresh_models')}
                            </Button>
                        }
                    />
                    <datalist id='lmstudio-models'>
                        {models.map((model) => (
                            <option
                                key={model}
                                value={model}
                            />
                        ))}
                    </datalist>
                </div>
                {modelError && <p className='text-small text-warning'>{modelError}</p>}
                {isLoadingModels && (
                    <p className='text-small text-default-500'>{t('services.translate.lmstudio.model_loading')}</p>
                )}

                <h3 className='my-auto'>Prompt List</h3>
                <p className='text-[10px] text-default-700'>{t('services.translate.lmstudio.prompt_description')}</p>
                <div className='bg-content2 rounded-[10px] p-3'>
                    {serviceConfig.promptList.map((prompt, index) => (
                        <div
                            className='config-item'
                            key={`${prompt.role}-${index}`}
                        >
                            <Textarea
                                label={prompt.role}
                                labelPlacement='outside'
                                variant='faded'
                                value={prompt.content}
                                placeholder={`Input Some ${prompt.role} Prompt`}
                                onValueChange={(value) => {
                                    setServiceConfig({
                                        ...serviceConfig,
                                        promptList: serviceConfig.promptList.map((item, promptIndex) =>
                                            promptIndex === index ? { ...item, content: value } : item
                                        ),
                                    });
                                }}
                            />
                            <Button
                                isIconOnly
                                color='danger'
                                className='my-auto mx-1'
                                variant='flat'
                                onPress={() => {
                                    setServiceConfig({
                                        ...serviceConfig,
                                        promptList: serviceConfig.promptList.filter(
                                            (_, promptIndex) => promptIndex !== index
                                        ),
                                    });
                                }}
                            >
                                <MdDeleteOutline className='text-[18px]' />
                            </Button>
                        </div>
                    ))}
                    <Button
                        fullWidth
                        onPress={() => {
                            setServiceConfig({
                                ...serviceConfig,
                                promptList: [
                                    ...serviceConfig.promptList,
                                    {
                                        role:
                                            serviceConfig.promptList.length === 0
                                                ? 'system'
                                                : serviceConfig.promptList.length % 2 === 0
                                                  ? 'assistant'
                                                  : 'user',
                                        content: '',
                                    },
                                ],
                            });
                        }}
                    >
                        {t('services.translate.lmstudio.add')}
                    </Button>
                </div>
                <br />

                <h3 className='my-auto'>Request Arguments</h3>
                <div className='config-item'>
                    <Textarea
                        label=''
                        labelPlacement='outside'
                        variant='faded'
                        value={serviceConfig.requestArguments}
                        placeholder='Input API Request Arguments'
                        onValueChange={(value) => {
                            setServiceConfig({ ...serviceConfig, requestArguments: value });
                        }}
                    />
                </div>
                <br />
                <Button
                    type='submit'
                    isLoading={isLoading}
                    fullWidth
                    color='primary'
                >
                    {t('common.save')}
                </Button>
            </form>
        )
    );
}
