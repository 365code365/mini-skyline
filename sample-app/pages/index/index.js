// 复杂演示页面
Page({
    data: {
        count: 0,
        todoCount: 3,
        todo1Done: true,
        todo2Done: false,
        todo3Done: false,
        selectedColor: 'green',
        currentTab: 0
    },
    
    onLoad: function() {
        __native_print('Page onLoad');
        this.updateTodoCount();
    },
    
    onShow: function() {
        __native_print('Page onShow');
    },
    
    // 计数器
    onIncrement: function() {
        this.setData({ count: this.data.count + 1 });
        __native_print('Count: ' + this.data.count);
    },
    
    onDecrement: function() {
        this.setData({ count: this.data.count - 1 });
        __native_print('Count: ' + this.data.count);
    },
    
    onReset: function() {
        this.setData({ count: 0 });
        __native_print('Count reset to 0');
    },
    
    // 待办事项
    onToggleTodo: function(e) {
        var id = e.currentTarget.dataset.id;
        __native_print('Toggle todo: ' + id);
        
        if (id === '1') {
            this.setData({ todo1Done: !this.data.todo1Done });
        } else if (id === '2') {
            this.setData({ todo2Done: !this.data.todo2Done });
        } else if (id === '3') {
            this.setData({ todo3Done: !this.data.todo3Done });
        }
        
        this.updateTodoCount();
    },
    
    updateTodoCount: function() {
        var done = 0;
        if (this.data.todo1Done) done++;
        if (this.data.todo2Done) done++;
        if (this.data.todo3Done) done++;
        this.setData({ todoCount: 3 - done });
        __native_print('Remaining todos: ' + (3 - done));
    },
    
    // 颜色选择
    onSelectColor: function(e) {
        var color = e.currentTarget.dataset.color;
        this.setData({ selectedColor: color });
        __native_print('Selected color: ' + color);
    },
    
    // 功能按钮
    onAction: function(e) {
        var type = e.currentTarget.dataset.type;
        __native_print('Action: ' + type);
        
        if (type === 'scan') {
            __native_print('Opening scanner...');
        } else if (type === 'pay') {
            __native_print('Opening payment...');
        } else if (type === 'card') {
            __native_print('Opening card wallet...');
        } else if (type === 'more') {
            __native_print('Opening settings...');
        }
    },
    
    // Tab 切换
    onSwitchTab: function(e) {
        var tab = parseInt(e.currentTarget.dataset.tab);
        __native_print('Switch to tab: ' + tab);
        this.setData({ currentTab: tab });
    },
    
    // 页面导航
    onGoList: function() {
        __native_print('Navigate to list page');
        wx.navigateTo({
            url: '/pages/list/list'
        });
    },
    
    onGoDetail: function() {
        __native_print('Navigate to detail page');
        wx.navigateTo({
            url: '/pages/detail/detail?id=1'
        });
    },
    
    // 退出登录
    onLogout: function() {
        __native_print('User logout');
        this.setData({
            count: 0,
            todoCount: 3,
            todo1Done: false,
            todo2Done: false,
            todo3Done: false,
            currentTab: 0
        });
        __native_print('Data reset');
    }
});
